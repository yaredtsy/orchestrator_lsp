//! The matching ladder (02-05 §4, VS-D44). Each node that is not where it was gets looked for in 6 steps; the first
//! step that finds it ends the search.
//!
//!   1 same path + name   2 same file content hash at a new path   3 git rename detection (-M)
//!   4 same body hash     5 similarity score                       6 nothing: Missing

use std::collections::{HashMap, HashSet};

use crate::git::Rename;
use crate::model::{Def, Node, Snapshot};
use crate::similarity::{Features, Fp, edit, features, jaccard};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Main {
    Jaccard,
    Edit,
}

/// VS-O10's knobs. A silent match needs ALL of: main score >= threshold, a margin over the second best,
/// the same file or the same class, and a body of at least `min_tokens`. Anything else asks the user.
#[derive(Debug, Clone)]
pub struct Config {
    pub main: Main,
    pub threshold: f32,
    pub margin: f32,
    pub min_tokens: usize,
    /// below this the best candidate is not even worth asking about
    pub ask_floor: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config { main: Main::Edit, threshold: 0.80, margin: 0.10, min_tokens: 20, ask_floor: 0.50 }
    }
}

#[derive(Debug, Clone)]
pub struct Cand {
    pub file: String,
    pub qualname: String,
    pub f: Features,
}

#[derive(Debug, Clone)]
pub enum Outcome {
    Found { step: u8, node: Node },
    /// Candidates the user is asked about ("Find it…"), best first.
    Ask(Vec<Cand>),
    Missing,
}

impl Outcome {
    pub fn short(&self) -> String {
        match self {
            Outcome::Found { step, node } => format!("step {step} -> {node}"),
            Outcome::Ask(c) => format!("ASK {}", c.iter().map(|c| format!("{}::{} (jac {:.2} ed {:.2})", c.file, c.qualname, c.f.jaccard, c.f.edit)).collect::<Vec<_>>().join(", ")),
            Outcome::Missing => "Missing".into(),
        }
    }
}

fn main_score(cfg: &Config, f: &Features) -> f32 {
    match cfg.main {
        Main::Jaccard => f.jaccard,
        Main::Edit => f.edit,
    }
}

pub fn locate(node: &Node, before: &Snapshot, after: &Snapshot, renames: &[Rename], m: &Matching, cfg: &Config) -> Outcome {
    match node {
        Node::Function { path, qualname } => finish(&stage_function(path, qualname, before, after, renames, m), cfg),
        Node::File { path } => locate_file(path, before, after, renames),
        Node::Folder { path } => locate_folder(path, before, after, renames),
    }
}

fn locate_file(path: &str, before: &Snapshot, after: &Snapshot, renames: &[Rename]) -> Outcome {
    if after.files.contains_key(path) {
        return Outcome::Found { step: 1, node: Node::File { path: path.into() } };
    }
    let Some(old) = before.files.get(path) else { return Outcome::Missing };
    if let Some((p, _)) = after.files.iter().find(|(p, f)| f.content_hash == old.content_hash && !before.files.contains_key(*p)) {
        return Outcome::Found { step: 2, node: Node::File { path: p.clone() } };
    }
    if let Some(r) = renames.iter().find(|r| r.old == path) {
        return Outcome::Found { step: 3, node: Node::File { path: r.new.clone() } };
    }
    Outcome::Missing
}

fn locate_folder(prefix: &str, before: &Snapshot, after: &Snapshot, renames: &[Rename]) -> Outcome {
    let dir = format!("{}/", prefix.trim_end_matches('/'));
    let kids: Vec<(&String, &crate::model::FileSnap)> = before.files.iter().filter(|(p, _)| p.starts_with(&dir)).collect();
    if kids.is_empty() {
        return Outcome::Missing;
    }
    if after.files.keys().any(|p| p.starts_with(&dir)) {
        return Outcome::Found { step: 1, node: Node::Folder { path: prefix.into() } };
    }
    // step 2: a folder matches by its children's hashes. Try the prefix implied by the first child.
    let (p0, f0) = kids[0];
    let rel0 = &p0[dir.len()..];
    for (q, fq) in &after.files {
        if fq.content_hash == f0.content_hash && q.ends_with(rel0) && q.len() > rel0.len() {
            let new_dir = &q[..q.len() - rel0.len()];
            let all = kids.iter().all(|(p, f)| after.files.get(&format!("{new_dir}{}", &p[dir.len()..])).is_some_and(|a| a.content_hash == f.content_hash));
            if all {
                return Outcome::Found { step: 2, node: Node::Folder { path: new_dir.trim_end_matches('/').into() } };
            }
        }
    }
    // step 3: git saw the children move to one common new folder
    let moved: Vec<&Rename> = renames.iter().filter(|r| r.old.starts_with(&dir)).collect();
    if let Some(first) = moved.first() {
        let rel = &first.old[dir.len()..];
        if first.new.ends_with(rel) && moved.len() * 2 >= kids.len() {
            let new_dir = &first.new[..first.new.len() - rel.len()];
            return Outcome::Found { step: 3, node: Node::Folder { path: new_dir.trim_end_matches('/').into() } };
        }
    }
    Outcome::Missing
}

fn find<'a>(s: &'a Snapshot, path: &str, qualname: &str) -> Option<&'a Def> {
    s.files.get(path)?.defs.iter().find(|d| d.qualname == qualname)
}

/// Who is still unmatched once steps 1-3 have run. Step 4 needs this: a body hash may only match silently when it is
/// unique among the unmatched OLD functions AND among the unmatched NEW ones. Two identical methods that were both
/// renamed in one commit are therefore never matched by hash alone.
pub struct Matching {
    old_hash: HashMap<String, usize>,
    new_hash: HashMap<String, usize>,
    /// (file, qualname) of new defs already claimed by steps 1-3
    claimed: HashSet<(String, String)>,
}

pub fn matching(before: &Snapshot, after: &Snapshot, renames: &[Rename]) -> Matching {
    let mut claimed: HashSet<(String, String)> = HashSet::new();
    let mut old_hash: HashMap<String, usize> = HashMap::new();
    for (path, file) in &before.files {
        // where did this file go? (step 2: same content hash at a new path; step 3: git rename detection)
        let mut homes: Vec<&str> = vec![path.as_str()];
        homes.extend(after.files.iter().filter(|(p, f)| *p != path && f.content_hash == file.content_hash && !before.files.contains_key(*p)).map(|(p, _)| p.as_str()));
        homes.extend(renames.iter().filter(|r| r.old == *path).map(|r| r.new.as_str()));
        for d in &file.defs {
            match homes.iter().find(|h| find(after, h, &d.qualname).is_some()) {
                Some(h) => {
                    claimed.insert((h.to_string(), d.qualname.clone()));
                }
                None => *old_hash.entry(d.body_hash.clone()).or_default() += 1,
            }
        }
    }
    let existed: HashSet<(&str, &str)> = before.all_defs().map(|d| (d.file.as_str(), d.qualname.as_str())).collect();
    let mut new_hash: HashMap<String, usize> = HashMap::new();
    for d in after.all_defs() {
        if !existed.contains(&(d.file.as_str(), d.qualname.as_str())) && !claimed.contains(&(d.file.clone(), d.qualname.clone())) {
            *new_hash.entry(d.body_hash.clone()).or_default() += 1;
        }
    }
    Matching { old_hash, new_hash, claimed }
}

/// What is left for steps 4 and 5.
pub struct Later<'a> {
    pub old: &'a Def,
    /// new, unmatched defs of the same kind
    pub pool: Vec<&'a Def>,
    pub mapped_file: String,
    /// step 4's silent candidate: only when the body hash is unique among unmatched old AND unmatched new defs
    pub exact_unique: Option<&'a Def>,
    /// how many unmatched old / new defs share the lost body hash (reported as the "twin" count)
    pub twins_old: usize,
    pub twins_new: usize,
}

pub enum Stage<'a> {
    Found { step: u8, node: Node },
    Missing,
    Later(Later<'a>),
}

/// Steps 1-3 for a function, then the inputs for steps 4-5. Nothing here depends on the thresholds.
pub fn stage_function<'a>(path: &str, qualname: &str, before: &'a Snapshot, after: &'a Snapshot, renames: &[Rename], m: &Matching) -> Stage<'a> {
    let found = |step, p: &str| Stage::Found { step, node: Node::Function { path: p.into(), qualname: qualname.into() } };
    // 1: same path and name
    if find(after, path, qualname).is_some() {
        return found(1, path);
    }
    let Some(old_file) = before.files.get(path) else { return Stage::Missing };
    let Some(old) = old_file.defs.iter().find(|d| d.qualname == qualname) else { return Stage::Missing };

    // 2: the whole file moved with identical content (also covers folder renames)
    let mut mapped_file = path.to_string();
    for (p, f) in &after.files {
        if p != path && f.content_hash == old_file.content_hash && !before.files.contains_key(p) {
            mapped_file = p.clone();
            if find(after, p, qualname).is_some() {
                return found(2, p);
            }
        }
    }
    // 3: git rename detection (the file moved AND was edited)
    if let Some(r) = renames.iter().find(|r| r.old == path) {
        mapped_file = r.new.clone();
        if find(after, &r.new, qualname).is_some() {
            return found(3, &r.new);
        }
    }

    // The candidates are the NEW, unmatched defs: one that was already there, or that steps 1-3 matched to something else, is not a successor.
    let existed: HashSet<(&str, &str)> = before.all_defs().map(|d| (d.file.as_str(), d.qualname.as_str())).collect();
    let pool: Vec<&Def> = after
        .all_defs()
        .filter(|d| d.kind == old.kind && !existed.contains(&(d.file.as_str(), d.qualname.as_str())) && !m.claimed.contains(&(d.file.clone(), d.qualname.clone())))
        .collect();
    let (twins_old, twins_new) = (m.old_hash.get(&old.body_hash).copied().unwrap_or(0), m.new_hash.get(&old.body_hash).copied().unwrap_or(0));
    let exact_unique = if twins_old == 1 && twins_new == 1 { pool.iter().copied().find(|d| d.body_hash == old.body_hash) } else { None };
    Stage::Later(Later { old, pool, mapped_file, exact_unique, twins_old, twins_new })
}

/// Steps 4 and 5, given the thresholds.
pub fn finish(stage: &Stage, cfg: &Config) -> Outcome {
    let l = match stage {
        Stage::Found { step, node } => return Outcome::Found { step: *step, node: node.clone() },
        Stage::Missing => return Outcome::Missing,
        Stage::Later(l) => l,
    };
    // 4: the same body (whitespace and comments stripped), UNIQUE among unmatched old and unmatched new defs, and long enough to mean something
    if let Some(d) = l.exact_unique.filter(|_| l.old.tokens.len() >= cfg.min_tokens) {
        return found_def(4, d);
    }
    // 5: similarity, with context. Duplicates land here too: identical candidates tie, a tie has no margin, so they ask.
    let scored = score_pool(l.old, &l.pool, &l.mapped_file);
    match choose(&scored, l.old.tokens.len(), cfg) {
        Choice::Silent(i) => Outcome::Found { step: 5, node: Node::Function { path: l.pool[scored[i].idx].file.clone(), qualname: l.pool[scored[i].idx].qualname.clone() } },
        Choice::Ask(ix) => Outcome::Ask(ix.into_iter().map(|i| Cand { file: l.pool[scored[i].idx].file.clone(), qualname: l.pool[scored[i].idx].qualname.clone(), f: scored[i].f.clone() }).collect()),
        Choice::None => Outcome::Missing,
    }
}

/// One scored candidate. Scoring is the expensive part and does not depend on the thresholds, so the tuning harness
/// scores once and then asks `choose` for many different `Config`s.
#[derive(Debug, Clone)]
pub struct Scored {
    pub idx: usize,
    pub f: Features,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Choice {
    /// index into the `scored` slice
    Silent(usize),
    Ask(Vec<usize>),
    None,
}

/// Scores the most promising candidates: the 12 best by Jaccard plus the 4 whose names are closest.
pub fn score_pool(lost_def: &Def, pool: &[&Def], mapped_file: &str) -> Vec<Scored> {
    let lost = Fp::new(lost_def);
    let fps: Vec<Fp> = pool.iter().map(|d| Fp::new(d)).collect();
    let mut by_jac: Vec<(usize, f32)> = fps.iter().enumerate().map(|(i, c)| (i, jaccard(&lost, c))).collect();
    by_jac.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut keep: Vec<usize> = by_jac.iter().take(12).map(|x| x.0).collect();
    let mut by_name: Vec<(usize, f32)> = fps.iter().enumerate().map(|(i, c)| (i, crate::similarity::name_sim(&lost, c))).collect();
    by_name.sort_by(|a, b| b.1.total_cmp(&a.1));
    keep.extend(by_name.iter().take(4).map(|x| x.0));
    keep.sort();
    keep.dedup();
    let _ = edit;
    keep.into_iter().map(|i| Scored { idx: i, f: features(&lost, &fps[i], mapped_file) }).collect()
}

/// The silent-match rule: ALL of main score >= threshold, a margin over the second best, same file or same class,
/// and a body of at least `min_tokens`. Otherwise ask (best first) if the best is worth asking about, else nothing.
pub fn choose(scored: &[Scored], lost_tokens: usize, cfg: &Config) -> Choice {
    let mut order: Vec<usize> = (0..scored.len()).collect();
    order.sort_by(|&a, &b| main_score(cfg, &scored[b].f).total_cmp(&main_score(cfg, &scored[a].f)).then(scored[b].f.name.total_cmp(&scored[a].f.name)));
    let Some(&best_i) = order.first() else { return Choice::None };
    let best = main_score(cfg, &scored[best_i].f);
    let second = order.get(1).map_or(0.0, |&i| main_score(cfg, &scored[i].f));
    let c = &scored[best_i].f;
    if best >= cfg.threshold && best - second >= cfg.margin && best - second > 1e-6 && (c.same_file || c.same_class) && lost_tokens >= cfg.min_tokens {
        return Choice::Silent(best_i);
    }
    if best >= cfg.ask_floor {
        return Choice::Ask(order.into_iter().take(3).collect());
    }
    Choice::None
}

fn found_def(step: u8, d: &Def) -> Outcome {
    Outcome::Found { step, node: Node::Function { path: d.file.clone(), qualname: d.qualname.clone() } }
}

