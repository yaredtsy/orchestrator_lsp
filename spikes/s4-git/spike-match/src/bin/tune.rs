//! S4 / VS-O10: tune the silent-match rule on a labelled set built from real functions of open-source repos.
//! Tune on one group of repos, validate on another, so the threshold does not just fit the data it was picked on.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use spike_match::driver::Driver;
use spike_match::git::{CliGit, GitBackend};
use spike_match::ladder::{Choice, Config, Main, Scored, Stage, choose, matching, score_pool, stage_function};
use spike_match::model::{Def, FileSnap, Snapshot};
use spike_match::similarity::{Fp, jaccard};

#[derive(Deserialize)]
struct Case {
    file: String,
    qualname: String,
    parent: String,
    parent_kind: Option<String>,
    new_name: String,
    level: f32,
    new_source: String,
}
#[derive(Deserialize)]
struct Dataset {
    cases: Vec<Case>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Place {
    SameScope,
    MovedFile,
    ModuleLevel,
    Deleted,
}

/// Everything a decision needs, computed once (scoring is the slow part, thresholds are cheap).
struct Prepared {
    level: f32,
    place: Place,
    lost_tokens: usize,
    /// pool index of the real successor (None = it was deleted)
    truth: Option<usize>,
    /// ladder step 4's silent candidate (pool index): the body hash is unique among unmatched old AND unmatched new defs
    exact_unique: Option<usize>,
    /// the corpus holds another def with exactly the lost body (a "twin")
    twin: bool,
    scored: Vec<Scored>,
    lost_q: String,
    pool_q: Vec<String>,
}

#[derive(Default, Clone, Debug)]
struct Counts {
    silent_right: u32,
    silent_wrong: u32,
    ask_top_right: u32,
    ask_in_list: u32,
    ask_no_truth: u32,
    missed: u32,
    deleted_asked: u32,
    deleted_ok: u32,
    /// cases whose lost function has an identical twin in the corpus (counted apart from the rest)
    twin_cases: u32,
    twin_wrong: u32,
}
impl Counts {
    fn total(&self) -> u32 {
        self.silent_right + self.silent_wrong + self.twin_wrong + self.ask_top_right + self.ask_in_list + self.ask_no_truth + self.missed + self.deleted_asked + self.deleted_ok
    }
    fn add(&mut self, o: &Counts) {
        self.silent_right += o.silent_right; self.silent_wrong += o.silent_wrong; self.ask_top_right += o.ask_top_right;
        self.ask_in_list += o.ask_in_list; self.ask_no_truth += o.ask_no_truth; self.missed += o.missed;
        self.deleted_asked += o.deleted_asked; self.deleted_ok += o.deleted_ok; self.twin_cases += o.twin_cases; self.twin_wrong += o.twin_wrong;
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

/// Mirrors `ladder::finish` for a stage that reached steps 4-5. Indices are pool indices.
enum D {
    Silent(usize),
    Ask(Vec<usize>),
    None,
}

fn decide(p: &Prepared, cfg: &Config) -> D {
    let d = decide_inner(p, cfg);
    // EXPERIMENT (S4_TWIN_NEVER_SILENT=1): when the lost body has an identical twin anywhere, only ask, never match silently
    if p.twin && std::env::var("S4_TWIN_NEVER_SILENT").map(|v| v == "1").unwrap_or(false) {
        if let D::Silent(i) = d { return D::Ask(vec![i]); }
    }
    d
}

fn decide_inner(p: &Prepared, cfg: &Config) -> D {
    if let Some(i) = p.exact_unique.filter(|_| p.lost_tokens >= cfg.min_tokens) {
        return D::Silent(i);
    }
    match choose(&p.scored, p.lost_tokens, cfg) {
        Choice::Silent(i) => D::Silent(p.scored[i].idx),
        Choice::Ask(v) => D::Ask(v.into_iter().map(|i| p.scored[i].idx).collect()),
        Choice::None => D::None,
    }
}

fn classify(p: &Prepared, cfg: &Config, c: &mut Counts) {
    // A wrong silent match in a TWIN case goes on its own line: the rest of the set is judged without it.
    let wrong = |c: &mut Counts| if p.twin { c.twin_wrong += 1 } else { c.silent_wrong += 1 };
    if p.twin { c.twin_cases += 1; }
    match (decide(p, cfg), p.truth) {
        (D::Silent(i), Some(t)) => if i == t { c.silent_right += 1 } else { wrong(c) },
        (D::Silent(_), None) => wrong(c),
        (D::Ask(idx), Some(t)) => {
            if idx.first() == Some(&t) { c.ask_top_right += 1 } else if idx.contains(&t) { c.ask_in_list += 1 } else { c.ask_no_truth += 1 }
        }
        (D::Ask(_), None) => c.deleted_asked += 1,
        (D::None, Some(_)) => c.missed += 1,
        (D::None, None) => c.deleted_ok += 1,
    }
}

fn load_corpus(git: &CliGit, driver: &mut Driver) -> anyhow::Result<Vec<Def>> {
    let mut all = vec![];
    for f in git.list_py_files("HEAD")? {
        if let Some(b) = git.read_file("HEAD", &f)? {
            all.extend(driver.defs(&f, &String::from_utf8_lossy(&b))?);
        }
    }
    Ok(all)
}

fn level_of(c: &Case) -> f32 { c.level }

/// One simulated commit as a REAL before/after pair, run through ladder steps 1-3 and the unmatched-uniqueness rule of step 4.
/// before = the lost function alone; after = the new defs only. (Every other function is unchanged, so steps 1-3 match it
/// and it takes no part: that is what makes twins that still exist harmless, and new copies of them real distractors.)
fn prepare_pair(lost: &Def, added: &[Def], successor: Option<&Def>, level: f32, place: Place, twin: bool) -> Prepared {
    let mut before = Snapshot::default();
    before.files.insert(lost.file.clone(), FileSnap { content_hash: format!("before:{}", lost.file), defs: vec![lost.clone()] });
    let mut after = Snapshot::default();
    for d in added {
        after.files.entry(d.file.clone()).or_insert_with(|| FileSnap { content_hash: format!("after:{}", d.file), defs: vec![] }).defs.push(d.clone());
    }
    let m = matching(&before, &after, &[]);
    let Stage::Later(later) = stage_function(&lost.file, &lost.qualname, &before, &after, &[], &m) else { unreachable!("a lost function never matches at steps 1-3 here") };
    let at = |d: &Def| later.pool.iter().position(|x| x.file == d.file && x.qualname == d.qualname);
    Prepared {
        level, place, lost_tokens: lost.tokens.len(),
        truth: successor.and_then(at),
        exact_unique: later.exact_unique.and_then(at),
        twin,
        scored: score_pool(lost, &later.pool, &later.mapped_file),
        lost_q: format!("{}::{}", lost.file, lost.qualname),
        pool_q: later.pool.iter().map(|d| format!("{}::{}", d.file, d.qualname)).collect(),
    }
}

fn prepare_repo(name: &str, repo: &Path, data: &Path, driver: &mut Driver) -> anyhow::Result<Vec<Prepared>> {
    let git = CliGit::new(repo);
    let corpus = load_corpus(&git, driver)?;
    let files: Vec<String> = { let mut f: Vec<String> = corpus.iter().map(|d| d.file.clone()).collect(); f.sort(); f.dedup(); f }; 
    let ds: Dataset = serde_json::from_slice(&std::fs::read(data.join(format!("{name}.json")))?)?;
    let mut out = vec![];
    let mut seen_deleted: std::collections::HashSet<(String, String)> = Default::default();
    let mut rng = Rng(0x9E3779B97F4A7C15 ^ name.len() as u64);
    for case in &ds.cases {
        let Some(lost) = corpus.iter().find(|d| d.file == case.file && d.qualname == case.qualname) else { continue };
        let mutated = driver.defs(&case.file, &case.new_source)?;
        let Some(m0) = mutated.iter().find(|d| d.qualname == case.new_name) else { continue };

        // distractors: other functions of the repo that count as NEW (renamed), plus the 3 closest siblings in the same file
        let lost_fp = Fp::new(lost);
        let others: Vec<&Def> = corpus.iter().filter(|d| d.kind == "function" && !(d.file == lost.file && d.qualname == lost.qualname)).collect();
        let mut distract: Vec<Def> = (0..25).map(|_| others[rng.below(others.len())].clone()).collect();
        let mut sib: Vec<(f32, &Def)> = others.iter().filter(|d| d.file == lost.file).map(|d| (jaccard(&lost_fp, &Fp::new(d)), *d)).collect();
        sib.sort_by(|a, b| b.0.total_cmp(&a.0));
        distract.extend(sib.iter().take(3).map(|s| s.1.clone()));
        for (i, d) in distract.iter_mut().enumerate() {
            d.qualname = format!("{}_n{i}", d.qualname); // new names: they did not exist before
            d.name = format!("{}_n{i}", d.name);
        }

        let places: Vec<Place> = {
            let mut v = vec![Place::SameScope, Place::MovedFile];
            if !lost.parent.is_empty() { v.push(Place::ModuleLevel); }
            v
        };
        let twin = corpus.iter().any(|d| !(d.file == lost.file && d.qualname == lost.qualname) && d.body_hash == lost.body_hash);
        for place in places {
            let mut m = m0.clone();
            m.file = case.file.clone();
            m.parent = case.parent.clone();
            m.parent_kind = case.parent_kind.clone();
            m.qualname = if case.parent.is_empty() { case.new_name.clone() } else { format!("{}.{}", case.parent, case.new_name) };
            match place {
                Place::MovedFile => {
                    let others_f: Vec<&String> = files.iter().filter(|f| **f != case.file).collect();
                    m.file = others_f[(rng.next() % others_f.len() as u64) as usize].clone();
                    m.parent = String::new(); m.parent_kind = None; m.qualname = case.new_name.clone();
                }
                Place::ModuleLevel => { m.parent = String::new(); m.parent_kind = None; m.qualname = case.new_name.clone(); }
                _ => {}
            }
            let mut added = distract.clone();
            added.push(m.clone());
            out.push(prepare_pair(lost, &added, Some(&m), level_of(case), place, twin));
        }
        // the deleted variant: once per lost function (no successor appears)
        if seen_deleted.insert((case.file.clone(), case.qualname.clone())) {
            out.push(prepare_pair(lost, &distract, None, -1.0, Place::Deleted, twin));
        }
    }
    eprintln!("{name}: {} cases", out.len());
    Ok(out)
}

fn run(set: &[Prepared], cfg: &Config) -> Counts {
    let mut c = Counts::default();
    for p in set { classify(p, cfg, &mut c); }
    c
}

fn main() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let data = root.join("data");
    let scratch = PathBuf::from(std::env::var("S4_SCRATCH").expect("set S4_SCRATCH to the folder holding httpx/requests/flask"));
    let lightrag = PathBuf::from(std::env::var("S4_LIGHTRAG").expect("set S4_LIGHTRAG"));
    let driver_bin = root.join("../s2-python-driver/spike-driver-python/target/release/spike-driver-python");
    let mut driver = Driver::start(&driver_bin)?;

    let mut train: Vec<Prepared> = vec![];
    let mut test: Vec<Prepared> = vec![];
    train.extend(prepare_repo("LightRAG", &lightrag, &data, &mut driver)?);
    train.extend(prepare_repo("httpx", &scratch.join("httpx"), &data, &mut driver)?);
    test.extend(prepare_repo("requests", &scratch.join("requests"), &data, &mut driver)?);
    test.extend(prepare_repo("flask", &scratch.join("flask"), &data, &mut driver)?);

    if std::env::var("S4_DIAG").is_ok() {
        let strict = Config { main: Main::Edit, threshold: 0.99, margin: 0.30, min_tokens: 50, ask_floor: 0.30 };
        let mut by: HashMap<(Place, bool), u32> = HashMap::new();
        let mut shown = 0;
        for p in train.iter() {
            let before = Counts::default();
            let mut c = before.clone();
            classify(p, &strict, &mut c);
            if c.silent_wrong + c.twin_wrong > 0 {
                *by.entry((p.place, p.exact_unique.is_some())).or_default() += 1;
                if shown < 12 {
                    shown += 1;
                    if let D::Silent(i) = decide(p, &strict) {
                        eprintln!("WRONG SILENT {:?} level {} via {} (twin case: {}): lost {} -> chose {} (truth {:?}) lost_tokens {}", p.place, p.level, if p.exact_unique.is_some() { "unique exact body hash" } else { "similarity" }, p.twin, p.lost_q, p.pool_q[i], p.truth.map(|t| p.pool_q[t].clone()), p.lost_tokens);
                    }
                }
            }
        }
        eprintln!("wrong silent at the strictest setting, by (placement, via unique exact body hash): {by:?}");
        return Ok(());
    }
    let mut md = String::new();
    writeln!(md, "# S4 tuning results\n\ntrain = LightRAG + httpx ({} cases), held-out = requests + flask ({} cases). ask_floor 0.30.\n", train.len(), test.len())?;

    // ── the grid ──
    let mut best: HashMap<&str, (Config, Counts)> = HashMap::new();
    for (label, main) in [("Edit", Main::Edit), ("Jaccard", Main::Jaccard)] {
        let mut ok: Vec<(Config, Counts)> = vec![];
        for t in (40..=99).map(|x| x as f32 / 100.0) {
            for margin in [0.0, 0.05, 0.10, 0.15, 0.20, 0.30] {
                for min_tokens in [10, 20, 30, 50] {
                    let cfg = Config { main, threshold: t, margin, min_tokens, ask_floor: 0.30 };
                    let c = run(&train, &cfg);
                    if c.silent_wrong == 0 { ok.push((cfg, c)); }
                }
            }
        }
        // most silent-right first; ties -> the most conservative (highest threshold, then margin, then min_tokens)
        ok.sort_by(|a, b| b.1.silent_right.cmp(&a.1.silent_right)
            .then(b.0.threshold.total_cmp(&a.0.threshold)).then(b.0.margin.total_cmp(&a.0.margin)).then(b.0.min_tokens.cmp(&a.0.min_tokens)));
        writeln!(md, "## {label} score: configs with ZERO wrong silent matches on the training set (twin cases judged apart): {} of {}\n", ok.len(), 60 * 6 * 4)?;
        writeln!(md, "| threshold | margin | min tokens | silent right | asked (top right) | asked (in list) | missed | deleted asked | twin cases | **twin cases silently WRONG** |\n|---|---|---|---|---|---|---|---|---|---|")?;
        for (cfg, c) in ok.iter().take(5) {
            writeln!(md, "| {:.2} | {:.2} | {} | {} | {} | {} | {} | {} | {} | **{}** |", cfg.threshold, cfg.margin, cfg.min_tokens, c.silent_right, c.ask_top_right, c.ask_in_list, c.missed, c.deleted_asked, c.twin_cases, c.twin_wrong)?;
        }
        if let Some(b) = ok.first() { best.insert(label, b.clone()); }
        writeln!(md)?;
    }

    // ── validate the winners on repos they were not tuned on, and show how close the cliff is ──
    writeln!(md, "## Held-out check (requests + flask)\n")?;
    writeln!(md, "| score | threshold | margin | min tokens | silent right | **silent WRONG** | twin cases (silently wrong) | asked | missed | wrong silent if threshold −0.02 / −0.05 / −0.10 (train+test) |\n|---|---|---|---|---|---|---|---|---|---|")?;
    for label in ["Edit", "Jaccard"] {
        let Some((cfg, _)) = best.get(label) else { continue };
        let c = run(&test, cfg);
        let near: Vec<u32> = [0.02, 0.05, 0.10].iter().map(|d| { let lo = Config { threshold: cfg.threshold - d, ..cfg.clone() }; { let (a, b) = (run(&train, &lo), run(&test, &lo)); a.silent_wrong + b.silent_wrong } }).collect();
        writeln!(md, "| {label} | {:.2} | {:.2} | {} | {} | **{}** | {} ({}) | {} | {} | {} / {} / {} |", cfg.threshold, cfg.margin, cfg.min_tokens, c.silent_right, c.silent_wrong, c.twin_cases, c.twin_wrong,
            c.ask_top_right + c.ask_in_list + c.ask_no_truth + c.deleted_asked, c.missed, near[0], near[1], near[2])?;
    }

    // ── breakdown for the winner of each score: by edit level and placement (train + held-out together) ──
    for label in ["Edit", "Jaccard"] {
        let Some((cfg, _)) = best.get(label) else { continue };
        writeln!(md, "\n## Breakdown, {label} score (threshold {:.2}, margin {:.2}, min tokens {}), train + held-out\n", cfg.threshold, cfg.margin, cfg.min_tokens)?;
        writeln!(md, "| edit level | placement | cases | silent right | asked | missed | wrong |\n|---|---|---|---|---|---|---|")?;
        for level in [0.0f32, 0.05, 0.10, 0.20, 0.35, 0.50, 0.70] {
            for place in [Place::SameScope, Place::ModuleLevel, Place::MovedFile] {
                let mut c = Counts::default();
                for p in train.iter().chain(test.iter()).filter(|p| p.place == place && (p.level - level).abs() < 1e-3) { classify(p, cfg, &mut c); }
                if c.total() == 0 { continue; }
                writeln!(md, "| {:.0}% | {:?} | {} | {} | {} | {} | {} |", level * 100.0, place, c.total(), c.silent_right, c.ask_top_right + c.ask_in_list + c.ask_no_truth, c.missed, c.silent_wrong)?;
            }
        }
        let mut c = Counts::default();
        for p in train.iter().chain(test.iter()).filter(|p| p.place == Place::Deleted) { classify(p, cfg, &mut c); }
        writeln!(md, "| (deleted) | Deleted | {} | silent {} | asked {} | – | – |", c.total(), c.silent_wrong, c.deleted_asked)?;
    }

    // ── what the features look like for the right answer vs the best wrong one, by edit level (same scope) ──
    writeln!(md, "\n## Features: the true successor vs the best WRONG candidate (same-scope placement, mean over cases)\n")?;
    writeln!(md, "| edit level | true: jaccard | true: edit | true: name | true: params | best wrong: jaccard | best wrong: edit | best wrong: name |\n|---|---|---|---|---|---|---|---|")?;
    for level in [0.0f32, 0.05, 0.10, 0.20, 0.35, 0.50, 0.70] {
        let (mut n, mut tj, mut te, mut tn, mut tp, mut wj, mut we, mut wn) = (0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32);
        for p in train.iter().chain(test.iter()).filter(|p| p.place == Place::SameScope && (p.level - level).abs() < 1e-3) {
            let truth = p.truth.unwrap();
            let Some(t) = p.scored.iter().find(|s| s.idx == truth) else { continue };
            let Some(w) = p.scored.iter().filter(|s| s.idx != truth).max_by(|a, b| a.f.edit.total_cmp(&b.f.edit)) else { continue };
            n += 1.0; tj += t.f.jaccard; te += t.f.edit; tn += t.f.name; tp += t.f.params; wj += w.f.jaccard; we += w.f.edit; wn += w.f.name;
        }
        if n > 0.0 { writeln!(md, "| {:.0}% | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |", level * 100.0, tj / n, te / n, tn / n, tp / n, wj / n, we / n, wn / n)?; }
    }
    print!("{md}");
    std::fs::write(data.join("tune-results.md"), &md)?;
    Ok(())
}
