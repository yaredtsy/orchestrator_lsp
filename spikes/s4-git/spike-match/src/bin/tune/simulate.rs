//! Turns a labelled case into a simulated commit: a REAL before/after pair that goes through ladder steps 1-3 and the
//! unmatched-uniqueness rule of step 4, so that what is left for steps 4-5 is exactly what the product would see.
//!
//! before = the lost function, plus its identical twins (if any)
//! after  = the twins unchanged (they match at step 1) + the NEW defs: the mutated successor and distractors
//! Every other function of the repo is unchanged, so steps 1-3 match it and it takes no part.

use std::collections::HashSet;

use spike_match::driver::Driver;
use spike_match::git::CliGit;
use spike_match::ladder::{Facts, Scored, Stage, Steps, matching, score_pool, stage_function};
use spike_match::model::{Def, FileSnap, Snapshot};
use spike_match::similarity::{Fp, jaccard};

use crate::dataset::{Repo, load_cases, load_corpus};

/// Where the mutated function lands in the after-state.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Place {
    SameScope,
    MovedFile,
    ModuleLevel,
    /// no successor appears at all
    Deleted,
}

/// Everything a decision needs, computed once (scoring is the slow part, thresholds are cheap).
pub struct Prepared {
    pub level: f32,
    pub place: Place,
    /// pool index of the real successor (None = it was deleted)
    pub truth: Option<usize>,
    /// step 4's candidate (pool index), when the body hash is unique among unmatched old and new defs
    pub exact_unique: Option<usize>,
    pub facts: Facts,
    pub scored: Vec<Scored>,
    pub lost_q: String,
    pub pool_q: Vec<String>,
}

pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn add_to(snap: &mut Snapshot, hash_prefix: &str, d: &Def) {
    snap.files.entry(d.file.clone()).or_insert_with(|| FileSnap { content_hash: format!("{hash_prefix}:{}", d.file), defs: vec![] }).defs.push(d.clone());
}

/// One simulated commit. `added` = the new defs; `successor` is the one of them that really is the lost function's successor.
pub fn prepare_pair(lost: &Def, twins: &[&Def], added: &[Def], successor: Option<&Def>, level: f32, place: Place) -> Prepared {
    let mut before = Snapshot::default();
    let mut after = Snapshot::default();
    add_to(&mut before, "before", lost);
    for t in twins {
        add_to(&mut before, "before", t);
        add_to(&mut after, "after", t); // unchanged: matches at step 1
    }
    for d in added {
        add_to(&mut after, "after", d);
    }
    let steps = Steps::all(); // the tuning measures steps 4-5, so they are on here
    let m = matching(&before, &after, &[], &steps);
    let Stage::Later(later) = stage_function(&lost.file, &lost.qualname, &before, &after, &[], &m, &steps) else {
        unreachable!("a lost function never matches at steps 1-3 here")
    };
    let at = |d: &Def| later.pool.iter().position(|x| x.file == d.file && x.qualname == d.qualname);
    Prepared {
        level,
        place,
        truth: successor.and_then(at),
        exact_unique: later.exact_unique,
        facts: later.facts(),
        scored: score_pool(lost, &later.pool, &later.mapped_file),
        lost_q: format!("{}::{}", lost.file, lost.qualname),
        pool_q: later.pool.iter().map(|d| format!("{}::{}", d.file, d.qualname)).collect(),
    }
}

pub fn prepare_repo(repo: &Repo, data: &std::path::Path, driver: &mut Driver) -> anyhow::Result<Vec<Prepared>> {
    let git = CliGit::new(&repo.path);
    let corpus = load_corpus(&git, driver)?;
    let files: Vec<String> = {
        let mut f: Vec<String> = corpus.iter().map(|d| d.file.clone()).collect();
        f.sort();
        f.dedup();
        f
    };
    let mut out = vec![];
    let mut seen_deleted: HashSet<(String, String)> = HashSet::new();
    let mut rng = Rng::new(0x9E3779B97F4A7C15 ^ repo.name.len() as u64);
    for case in load_cases(data, repo.name)? {
        let Some(lost) = corpus.iter().find(|d| d.file == case.file && d.qualname == case.qualname) else { continue };
        let mutated = driver.defs(&case.file, &case.new_source)?;
        let Some(m0) = mutated.iter().find(|d| d.qualname == case.new_name) else { continue };

        // Distractors: other functions of the repo, copied under NEW names, plus the 3 closest siblings in the same file.
        // The originals stay where they are (they match at step 1), so these are genuinely new functions.
        let lost_fp = Fp::new(lost);
        let others: Vec<&Def> = corpus.iter().filter(|d| d.kind == "function" && !(d.file == lost.file && d.qualname == lost.qualname)).collect();
        let mut distract: Vec<Def> = (0..25).map(|_| others[rng.below(others.len())].clone()).collect();
        let mut sib: Vec<(f32, &Def)> = others.iter().filter(|d| d.file == lost.file).map(|d| (jaccard(&lost_fp, &Fp::new(d)), *d)).collect();
        sib.sort_by(|a, b| b.0.total_cmp(&a.0));
        distract.extend(sib.iter().take(3).map(|s| s.1.clone()));
        for (i, d) in distract.iter_mut().enumerate() {
            d.qualname = format!("{}_n{i}", d.qualname);
            d.name = format!("{}_n{i}", d.name);
        }
        let twins: Vec<&Def> = others.iter().copied().filter(|d| d.body_hash == lost.body_hash).collect();

        let mut places = vec![Place::SameScope, Place::MovedFile];
        if !lost.parent.is_empty() {
            places.push(Place::ModuleLevel);
        }
        for place in places {
            let mut m = m0.clone();
            m.file = case.file.clone();
            m.parent = case.parent.clone();
            m.parent_kind = case.parent_kind.clone();
            m.qualname = if case.parent.is_empty() { case.new_name.clone() } else { format!("{}.{}", case.parent, case.new_name) };
            match place {
                Place::MovedFile => {
                    let other_files: Vec<&String> = files.iter().filter(|f| **f != case.file).collect();
                    m.file = other_files[rng.below(other_files.len())].clone();
                    m.parent = String::new();
                    m.parent_kind = None;
                    m.qualname = case.new_name.clone();
                }
                Place::ModuleLevel => {
                    m.parent = String::new();
                    m.parent_kind = None;
                    m.qualname = case.new_name.clone();
                }
                _ => {}
            }
            let mut added = distract.clone();
            added.push(m.clone());
            out.push(prepare_pair(lost, &twins, &added, Some(&m), case.level, place));
        }
        // the deleted variant: once per lost function (no successor appears)
        if seen_deleted.insert((case.file.clone(), case.qualname.clone())) {
            out.push(prepare_pair(lost, &twins, &distract, None, -1.0, Place::Deleted));
        }
    }
    eprintln!("{}: {} cases", repo.name, out.len());
    Ok(out)
}
