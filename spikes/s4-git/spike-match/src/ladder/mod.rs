//! The matching ladder (02-05 §4, VS-D44). A documented node that is not where it was gets looked for in six steps;
//! the first step that finds it ends the search. Steps 4 and 5 hand their candidates to the policy, which picks a tier.
//! **The MVP runs step 1 only**; steps 2-5 are behind `Config::steps` (`Steps`), off by default.
//!
//!   1 same path + name          function.rs / nodes.rs
//!   2 same file content hash    function.rs / nodes.rs   (also folder renames)
//!   3 git rename detection      function.rs / nodes.rs   (file moved AND edited)
//!   4 same body hash            policy::step4            (unique among unmatched old and new: matching.rs)
//!   5 similarity, with context  rank.rs scores, policy::step5 picks the tier
//!   6 nothing                   Missing
//!
//! Where to change what:
//!   a threshold or a new rule group   config.rs
//!   the tier rules                    policy.rs  (pure functions, table-driven tests)
//!   a new score or feature            ../similarity.rs, then rank.rs
//!   what the user is shown            outcome.rs

mod config;
mod function;
mod matching;
mod nodes;
mod outcome;
mod policy;
mod rank;

pub use config::{AskRules, Config, Main, ProvisionalRule, SilentRule, Steps};
pub use function::{Later, Stage, finish, stage_function};
pub use matching::{Matching, matching};
pub use outcome::{AskReason, Cand, Outcome};
pub use policy::{Facts, Verdict, ask_only_reasons, resolve, step4, step5};
pub use rank::{Lead, Scored, lead, order, score_pool};

use crate::git::Rename;
use crate::model::{Node, Snapshot};

/// One node, start to finish. (`matching` is computed once per before/after pair and shared by all nodes.)
pub fn locate(node: &Node, before: &Snapshot, after: &Snapshot, renames: &[Rename], m: &Matching, cfg: &Config) -> Outcome {
    match node {
        Node::Function { path, qualname } => finish(&stage_function(path, qualname, before, after, renames, m, &cfg.steps), cfg),
        Node::File { path } => nodes::locate_file(path, before, after, renames, &cfg.steps),
        Node::Folder { path } => nodes::locate_folder(path, before, after, renames, &cfg.steps),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{def, snap};

    const BODY: &str = "total = 0 . 0 for item in items : total += item [ 'p' ] return round ( total , 2 ) + 1 + 2 + 3 + 4";
    fn node() -> Node {
        Node::Function { path: "a.py".into(), qualname: "f".into() }
    }
    fn run(before: &Snapshot, after: &Snapshot, renames: &[Rename], cfg: &Config) -> Outcome {
        locate(&node(), before, after, renames, &matching(before, after, renames, &cfg.steps), cfg)
    }

    #[test]
    fn the_mvp_finds_a_function_by_path_and_name() {
        let b = snap(&[("a.py", "h1", vec![def("a.py", "f", BODY)])]);
        let a = snap(&[("a.py", "h2", vec![def("a.py", "f", "return 0")])]); // even with a different body: step 1 is path + name
        assert!(matches!(run(&b, &a, &[], &Config::default()), Outcome::Found { step: 1, .. }));
    }

    #[test]
    fn the_mvp_does_not_follow_a_moved_file() {
        let b = snap(&[("a.py", "h1", vec![def("a.py", "f", BODY)])]);
        let a = snap(&[("b.py", "h1", vec![def("b.py", "f", BODY)])]); // same content, new path
        let moved = Rename { old: "a.py".into(), new: "b.py".into(), score: 100 };
        assert!(matches!(run(&b, &a, &[moved.clone()], &Config::default()), Outcome::Missing), "MVP: steps 2-3 are off");
        assert!(matches!(run(&b, &a, &[moved], &Config::default().with_steps(Steps::all())), Outcome::Found { step: 2, .. }), "and on when asked");
    }

    #[test]
    fn the_mvp_does_not_follow_a_renamed_function() {
        let b = snap(&[("a.py", "h1", vec![def("a.py", "f", BODY)])]);
        let a = snap(&[("a.py", "h2", vec![def("a.py", "g", BODY)])]);
        assert!(matches!(run(&b, &a, &[], &Config::default()), Outcome::Missing), "MVP: steps 4-5 are off");
        assert!(matches!(run(&b, &a, &[], &Config::default().with_steps(Steps::all())), Outcome::Found { step: 4, .. }));
    }

    #[test]
    fn a_lost_id_comment_is_step_1_if_the_function_is_still_there() {
        // MVP rule 6: same path::qualname still exists -> found by step 1 (the product then offers to restore the ID)
        let b = snap(&[("a.py", "h1", vec![def("a.py", "C.f", BODY)])]);
        let a = snap(&[("a.py", "h2", vec![def("a.py", "C.f", BODY)])]);
        let n = Node::Function { path: "a.py".into(), qualname: "C.f".into() };
        let cfg = Config::default();
        assert!(matches!(locate(&n, &b, &a, &[], &matching(&b, &a, &[], &cfg.steps), &cfg), Outcome::Found { step: 1, .. }));
    }

    #[test]
    fn files_and_folders_are_step_1_only_in_the_mvp() {
        let b = snap(&[("d/a.py", "h1", vec![])]);
        let a = snap(&[("e/a.py", "h1", vec![])]);
        let cfg = Config::default();
        let m = matching(&b, &a, &[], &cfg.steps);
        assert!(matches!(locate(&Node::File { path: "d/a.py".into() }, &b, &a, &[], &m, &cfg), Outcome::Missing));
        assert!(matches!(locate(&Node::Folder { path: "d".into() }, &b, &a, &[], &m, &cfg), Outcome::Missing));
        let all = Config::default().with_steps(Steps::all());
        assert!(matches!(locate(&Node::Folder { path: "d".into() }, &b, &a, &[], &m, &all), Outcome::Found { step: 2, .. }));
    }
}
