//! Functions: steps 1-3 here (`stage_function`), then the hand-off to the policy for steps 4-5 (`finish`).
//! `stage_function` does not depend on any threshold, so the tuning harness runs it once per case.

use std::collections::HashSet;

use crate::git::Rename;
use crate::model::{Def, Node, Snapshot};

use super::config::{Config, Steps};
use super::matching::{Matching, find};
use super::outcome::{Cand, Outcome};
use super::policy::{Facts, Verdict, resolve};
use super::rank::{Scored, score_pool};

/// What is left for steps 4 and 5.
pub struct Later<'a> {
    pub old: &'a Def,
    /// new, unmatched defs of the same kind (the candidates)
    pub pool: Vec<&'a Def>,
    pub mapped_file: String,
    /// step 4's candidate, as a pool index: only when the body hash is unique among unmatched old AND unmatched new defs
    pub exact_unique: Option<usize>,
    /// another function in the before-state has exactly the lost body
    pub has_twin: bool,
}

impl Later<'_> {
    pub fn facts(&self) -> Facts {
        Facts { lost_tokens: self.old.tokens.len(), has_twin: self.has_twin }
    }
}

pub enum Stage<'a> {
    Found { step: u8, node: Node },
    Missing,
    Later(Later<'a>),
}

/// Steps 1-3 for a function, then the inputs for steps 4-5.
pub fn stage_function<'a>(path: &str, qualname: &str, before: &'a Snapshot, after: &'a Snapshot, renames: &[Rename], m: &Matching, steps: &Steps) -> Stage<'a> {
    let found = |step, p: &str| Stage::Found { step, node: Node::Function { path: p.into(), qualname: qualname.into() } };
    // 1: same path and name
    if find(after, path, qualname).is_some() {
        return found(1, path);
    }
    let Some(old_file) = before.files.get(path) else { return Stage::Missing };
    let Some(old) = old_file.defs.iter().find(|d| d.qualname == qualname) else { return Stage::Missing };

    // 2: the whole file moved with identical content (also covers folder renames)
    let mut mapped_file = path.to_string();
    for (p, f) in after.files.iter().filter(|_| steps.file_moves) {
        if p != path && f.content_hash == old_file.content_hash && !before.files.contains_key(p) {
            mapped_file = p.clone();
            if find(after, p, qualname).is_some() {
                return found(2, p);
            }
        }
    }
    // 3: git rename detection (the file moved AND was edited)
    if let Some(r) = renames.iter().filter(|_| steps.file_moves).find(|r| r.old == path) {
        mapped_file = r.new.clone();
        if find(after, &r.new, qualname).is_some() {
            return found(3, &r.new);
        }
    }

    // The candidates are the NEW, unmatched defs: one that was already there, or that steps 1-3 matched to something else, is not a successor.
    let existed: HashSet<(&str, &str)> = before.all_defs().map(|d| (d.file.as_str(), d.qualname.as_str())).collect();
    let pool: Vec<&Def> = after
        .all_defs()
        .filter(|d| d.kind == old.kind && !existed.contains(&(d.file.as_str(), d.qualname.as_str())) && !m.is_claimed(&d.file, &d.qualname))
        .collect();
    let exact_unique = if m.is_unique(&old.body_hash) { pool.iter().position(|d| d.body_hash == old.body_hash) } else { None };
    let has_twin = has_twin(old, before, after);
    Stage::Later(Later { old, pool, mapped_file, exact_unique, has_twin })
}

/// An exact twin of the lost body, anywhere in the before-state or the after-state, matched or unmatched (VS-D73).
/// Before: any OTHER function with the same body. After: a body that now appears twice or more (once is the candidate itself,
/// which is what a clean rename looks like).
fn has_twin(old: &Def, before: &Snapshot, after: &Snapshot) -> bool {
    let in_before = before.all_defs().any(|d| !(d.file == old.file && d.qualname == old.qualname) && d.body_hash == old.body_hash);
    let in_after = after.all_defs().filter(|d| d.body_hash == old.body_hash).count() >= 2;
    in_before || in_after
}

/// Steps 4 and 5, given the config. Scoring (the slow part) is skipped when step 4 already decides.
pub fn finish(stage: &Stage, cfg: &Config) -> Outcome {
    let l = match stage {
        Stage::Found { step, node } => return Outcome::Found { step: *step, node: node.clone() },
        Stage::Missing => return Outcome::Missing,
        Stage::Later(l) => l,
    };
    let facts = l.facts();
    let (verdict, scored) = match super::policy::step4(l.exact_unique, &facts, cfg) {
        Some(v) => (v, Vec::new()),
        None => {
            let scored = if cfg.steps.similarity { score_pool(l.old, &l.pool, &l.mapped_file) } else { Vec::new() };
            (resolve(None, &scored, &facts, cfg), scored)
        }
    };
    to_outcome(l, &scored, verdict)
}

fn cand(l: &Later, scored: &[Scored], pool: usize) -> Cand {
    let d = l.pool[pool];
    let f = scored.iter().find(|s| s.idx == pool).map(|s| s.f.clone()).expect("a candidate in a verdict was scored");
    Cand { file: d.file.clone(), qualname: d.qualname.clone(), f }
}

fn to_outcome(l: &Later, scored: &[Scored], v: Verdict) -> Outcome {
    match v {
        Verdict::Silent { pool, step } => Outcome::Found { step, node: Node::Function { path: l.pool[pool].file.clone(), qualname: l.pool[pool].qualname.clone() } },
        Verdict::Provisional { pool } => Outcome::Provisional(cand(l, scored, pool)),
        Verdict::Ask { pool, reasons } => Outcome::Ask { candidates: pool.into_iter().map(|p| cand(l, scored, p)).collect(), reasons },
        Verdict::Nothing => Outcome::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ladder::matching::matching;
    use crate::testutil::{def, snap};

    const BODY: &str = "total = 0 . 0 for item in items : total += item [ 'p' ] return round ( total , 2 )";
    const OTHER: &str = "return a + b";

    fn twin_of(before: &Snapshot, after: &Snapshot) -> bool {
        let m = matching(before, after, &[], &Steps::all());
        match stage_function("a.py", "C.gross", before, after, &[], &m, &Steps::all()) {
            Stage::Later(l) => l.has_twin,
            _ => panic!("expected the function to be left for steps 4-5"),
        }
    }

    #[test]
    fn a_clean_rename_is_not_a_twin_case() {
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY)])]);
        assert!(!twin_of(&before, &after));
    }

    #[test]
    fn an_unrelated_function_with_another_body_is_not_a_twin() {
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY), def("a.py", "C.add", OTHER)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY), def("a.py", "C.add", OTHER)])]);
        assert!(!twin_of(&before, &after));
    }

    #[test]
    fn a_matched_twin_counts() {
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY), def("a.py", "C.net", BODY)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY), def("a.py", "C.net", BODY)])]);
        assert!(twin_of(&before, &after));
    }

    #[test]
    fn an_unmatched_twin_counts() {
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY), def("a.py", "C.net", BODY)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY), def("a.py", "C.net_total", BODY)])]);
        assert!(twin_of(&before, &after));
    }

    #[test]
    fn a_new_copy_in_the_after_state_counts() {
        // nothing like it before, but the body now exists twice
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY), def("b.py", "copy_of_it", BODY)])]);
        assert!(twin_of(&before, &after));
    }
}
