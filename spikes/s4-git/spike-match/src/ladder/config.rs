//! Every knob of the ladder, in one place. Each rule group is its own small struct, so a new rule (or a new tier)
//! adds a struct here and one branch in `policy.rs`, and nothing else has to change.

use crate::similarity::Features;

/// Which body score drives the thresholds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Main {
    Jaccard,
    Edit,
}

impl Main {
    pub fn of(self, f: &Features) -> f32 {
        match self {
            Main::Jaccard => f.jaccard,
            Main::Edit => f.edit,
        }
    }
}

/// Tier 1, a silent match: the link is made without asking and without a label.
/// Also needs the best candidate to be in the same file or the same class (see `policy::is_silent`).
#[derive(Debug, Clone)]
pub struct SilentRule {
    pub threshold: f32,
    pub margin: f32,
}

/// Tier 2, a provisional match: the link is made but shown as "Unconfirmed link" until the user confirms it.
/// Below the silent threshold, with a clear margin over the second best.
#[derive(Debug, Clone)]
pub struct ProvisionalRule {
    pub floor: f32,
    pub margin: f32,
}

/// Rules that force a question, whatever the score: they cap the answer at tier 3.
#[derive(Debug, Clone)]
pub struct AskRules {
    /// bodies shorter than this never match without asking
    pub min_tokens: usize,
    /// a lost body with an identical twin anywhere never matches without asking
    pub twins: bool,
    /// below this the best candidate is not even worth asking about (the node is Missing)
    pub floor: f32,
}

/// Which ladder steps run. Step 1 (same `path::qualname`) always runs. The MVP uses step 1 only; the others are built and
/// tested but off until the features that need them ship (no-ID mode, step 0 and the matching steps come together).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Steps {
    /// steps 2 and 3: the file moved (same content hash, or git's rename detection)
    pub file_moves: bool,
    /// step 4: the same body under another name or in another file
    pub body_hash: bool,
    /// step 5: similarity, with context (and the provisional tier, which is a tier of step 5)
    pub similarity: bool,
}

impl Steps {
    /// The MVP: step 1 only.
    pub const fn mvp() -> Self {
        Steps { file_moves: false, body_hash: false, similarity: false }
    }
    pub const fn all() -> Self {
        Steps { file_moves: true, body_hash: true, similarity: true }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub steps: Steps,
    pub score: Main,
    pub silent: SilentRule,
    /// `None` = the tier is off (silent or ask, as before)
    pub provisional: Option<ProvisionalRule>,
    pub ask: AskRules,
}

impl Default for Config {
    /// Starting values, not tuned values: the tuned ones come from `tune` (spike S4).
    fn default() -> Self {
        Config {
            steps: Steps::mvp(),
            score: Main::Edit,
            silent: SilentRule { threshold: 0.80, margin: 0.10 },
            provisional: None,
            ask: AskRules { min_tokens: 20, twins: true, floor: 0.50 },
        }
    }
}

impl Config {
    pub fn with_steps(mut self, steps: Steps) -> Self {
        self.steps = steps;
        self
    }
    pub fn with_score(mut self, score: Main) -> Self {
        self.score = score;
        self
    }
    pub fn with_silent(mut self, threshold: f32, margin: f32) -> Self {
        self.silent = SilentRule { threshold, margin };
        self
    }
    pub fn with_provisional(mut self, floor: f32, margin: f32) -> Self {
        self.provisional = Some(ProvisionalRule { floor, margin });
        self
    }
    pub fn with_ask(mut self, min_tokens: usize, twins: bool, floor: f32) -> Self {
        self.ask = AskRules { min_tokens, twins, floor };
        self
    }
}
