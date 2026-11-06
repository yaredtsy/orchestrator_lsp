//! The matching ladder (02-05 §4, VS-D44). A documented node that is not where it was gets looked for in six steps;
//! the first step that finds it ends the search. Steps 4 and 5 hand their candidates to the policy, which picks a tier.
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

pub use config::{AskRules, Config, Main, ProvisionalRule, SilentRule};
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
        Node::Function { path, qualname } => finish(&stage_function(path, qualname, before, after, renames, m), cfg),
        Node::File { path } => nodes::locate_file(path, before, after, renames),
        Node::Folder { path } => nodes::locate_folder(path, before, after, renames),
    }
}
