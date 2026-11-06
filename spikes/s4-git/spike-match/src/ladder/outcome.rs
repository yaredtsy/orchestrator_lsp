//! What the ladder tells the rest of the product about one node.

use crate::model::Node;
use crate::similarity::Features;

/// Why a candidate list is a question and not a link. Shown to the user ("an identical twin exists") and counted by the tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AskReason {
    /// the lost body has an identical twin: the match could be either
    Twin,
    /// the body is too short to mean anything
    ShortBody,
    /// two candidates score about the same, or the context (file / class) does not agree
    Ambiguous,
    /// the best candidate scores below the provisional floor (or provisional is off)
    Weak,
}

#[derive(Debug, Clone)]
pub struct Cand {
    pub file: String,
    pub qualname: String,
    pub f: Features,
}

impl Cand {
    pub fn label(&self) -> String {
        format!("{}::{}", self.file, self.qualname)
    }
}

#[derive(Debug, Clone)]
pub enum Outcome {
    /// tier 1: found, no question, no label
    Found { step: u8, node: Node },
    /// tier 2: linked, shown as "Unconfirmed link" until the user confirms (the ladder is deterministic, so teammates compute the same link)
    Provisional(Cand),
    /// tier 3: "Find it…", best first
    Ask { candidates: Vec<Cand>, reasons: Vec<AskReason> },
    /// nothing to offer
    Missing,
}

impl Outcome {
    pub fn short(&self) -> String {
        let one = |c: &Cand| format!("{} (jac {:.2} ed {:.2})", c.label(), c.f.jaccard, c.f.edit);
        match self {
            Outcome::Found { step, node } => format!("step {step} -> {node}"),
            Outcome::Provisional(c) => format!("PROVISIONAL {}", one(c)),
            Outcome::Ask { candidates, reasons } => format!("ASK {:?}: {}", reasons, candidates.iter().map(one).collect::<Vec<_>>().join(", ")),
            Outcome::Missing => "Missing".into(),
        }
    }
}
