//! The tier decision: given scored candidates and a few facts about the lost function, is this a silent link,
//! a provisional link, a question, or nothing?
//!
//!   tier 1  Silent       linked, no label
//!   tier 2  Provisional  linked, shown as "Unconfirmed link" until the user confirms
//!   tier 3  Ask          "Find it…" with a ranked list and the reasons
//!           Nothing      the node stays Missing
//!
//! This is the file to change when the product rule changes. Everything here is a pure function of its inputs, so each
//! rule has a table-driven test at the bottom. The tuning harness calls `resolve` too: it must never re-implement the rule.

use super::config::Config;
use super::outcome::AskReason;
use super::rank::{Lead, Scored, lead, order};

/// What is known about the lost function itself, whatever the candidates are.
#[derive(Debug, Clone, Copy)]
pub struct Facts {
    pub lost_tokens: usize,
    /// another function (before the change) has exactly this body
    pub has_twin: bool,
}

/// Every index is a POOL index (`Scored::idx`), never a position in the `scored` slice.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Silent { pool: usize, step: u8 },
    Provisional { pool: usize },
    Ask { pool: Vec<usize>, reasons: Vec<AskReason> },
    Nothing,
}

/// A tie has no margin: two candidates scoring the same can never be told apart by score.
const TIE: f32 = 1e-6;

/// Rules that cap the answer at "ask", whatever the score. Add a new one here and it applies to steps 4 and 5 and both upper tiers.
pub fn ask_only_reasons(facts: &Facts, cfg: &Config) -> Vec<AskReason> {
    let mut r = Vec::new();
    if cfg.ask.twins && facts.has_twin {
        r.push(AskReason::Twin);
    }
    if facts.lost_tokens < cfg.ask.min_tokens {
        r.push(AskReason::ShortBody);
    }
    r
}

/// Step 4: the same body, unique among unmatched old and unmatched new functions (the caller computed that).
/// `None` = step 4 does not decide; go on to step 5.
pub fn step4(exact_unique: Option<usize>, facts: &Facts, cfg: &Config) -> Option<Verdict> {
    let pool = exact_unique?;
    ask_only_reasons(facts, cfg).is_empty().then_some(Verdict::Silent { pool, step: 4 })
}

fn is_silent(lead: &Lead, best: &Scored, cfg: &Config) -> bool {
    lead.best >= cfg.silent.threshold && lead.margin >= cfg.silent.margin && lead.margin > TIE && (best.f.same_file || best.f.same_class)
}

fn is_provisional(lead: &Lead, cfg: &Config) -> bool {
    cfg.provisional.as_ref().is_some_and(|p| lead.best >= p.floor && lead.margin >= p.margin && lead.margin > TIE)
}

/// Step 5: similarity, with context.
pub fn step5(scored: &[Scored], facts: &Facts, cfg: &Config) -> Verdict {
    let Some(lead) = lead(scored, cfg.score) else { return Verdict::Nothing };
    let mut reasons = ask_only_reasons(facts, cfg);
    if reasons.is_empty() {
        let best = &scored[lead.pos];
        if is_silent(&lead, best, cfg) {
            return Verdict::Silent { pool: best.idx, step: 5 };
        }
        if is_provisional(&lead, cfg) {
            return Verdict::Provisional { pool: best.idx };
        }
    }
    if lead.best >= cfg.ask.floor {
        if reasons.is_empty() {
            // why it is only a question: the score is decent but the lead or the context is not
            let strong = lead.best >= cfg.provisional.as_ref().map_or(cfg.silent.threshold, |p| p.floor);
            reasons.push(if strong { AskReason::Ambiguous } else { AskReason::Weak });
        }
        let pool = order(scored, cfg.score).into_iter().take(3).map(|p| scored[p].idx).collect();
        return Verdict::Ask { pool, reasons };
    }
    Verdict::Nothing
}

/// Steps 4 and 5 together.
pub fn resolve(exact_unique: Option<usize>, scored: &[Scored], facts: &Facts, cfg: &Config) -> Verdict {
    step4(exact_unique, facts, cfg).unwrap_or_else(|| step5(scored, facts, cfg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ladder::config::Config;
    use crate::similarity::Features;

    /// (pool index, edit score, same_file); jaccard = edit, name 0.5
    fn scored(v: &[(usize, f32, bool)]) -> Vec<Scored> {
        v.iter()
            .map(|&(idx, e, same_file)| Scored { idx, f: Features { jaccard: e, edit: e, name: 0.5, params: 0.5, same_file, same_class: false } })
            .collect()
    }
    const OK: Facts = Facts { lost_tokens: 100, has_twin: false };

    fn cfg() -> Config {
        Config::default().with_silent(0.95, 0.10).with_provisional(0.70, 0.10).with_ask(20, true, 0.40)
    }

    #[test]
    fn tiers() {
        let c = cfg();
        // (name, candidates, facts, expected)
        let cases: Vec<(&str, Vec<Scored>, Facts, Verdict)> = vec![
            ("clear winner in the same file is silent", scored(&[(7, 0.98, true), (8, 0.50, true)]), OK, Verdict::Silent { pool: 7, step: 5 }),
            ("a tie is never silent", scored(&[(7, 0.98, true), (8, 0.98, true)]), OK, Verdict::Ask { pool: vec![7, 8], reasons: vec![AskReason::Ambiguous] }),
            ("high score but another file is provisional, not silent", scored(&[(7, 0.98, false), (8, 0.50, false)]), OK, Verdict::Provisional { pool: 7 }),
            ("good score, clear lead, below silent: provisional", scored(&[(7, 0.80, true), (8, 0.40, true)]), OK, Verdict::Provisional { pool: 7 }),
            ("good score but a close second: ask", scored(&[(7, 0.80, true), (8, 0.75, true)]), OK, Verdict::Ask { pool: vec![7, 8], reasons: vec![AskReason::Ambiguous] }),
            ("below the provisional floor: ask, weak", scored(&[(7, 0.55, true), (8, 0.20, true)]), OK, Verdict::Ask { pool: vec![7, 8], reasons: vec![AskReason::Weak] }),
            ("below the ask floor: nothing", scored(&[(7, 0.30, true)]), OK, Verdict::Nothing),
            ("no candidates: nothing", vec![], OK, Verdict::Nothing),
            ("a twin caps it at ask, even at 1.0", scored(&[(7, 1.0, true), (8, 0.2, true)]), Facts { has_twin: true, ..OK }, Verdict::Ask { pool: vec![7, 8], reasons: vec![AskReason::Twin] }),
            ("a short body caps it at ask", scored(&[(7, 1.0, true), (8, 0.2, true)]), Facts { lost_tokens: 5, ..OK }, Verdict::Ask { pool: vec![7, 8], reasons: vec![AskReason::ShortBody] }),
            ("a twin AND a short body: both reasons", scored(&[(7, 1.0, true)]), Facts { lost_tokens: 5, has_twin: true }, Verdict::Ask { pool: vec![7], reasons: vec![AskReason::Twin, AskReason::ShortBody] }),
        ];
        for (name, s, f, want) in cases {
            assert_eq!(resolve(None, &s, &f, &c), want, "{name}");
        }
    }

    #[test]
    fn provisional_off_means_ask() {
        let mut c = cfg();
        c.provisional = None;
        let s = scored(&[(7, 0.80, true), (8, 0.40, true)]);
        assert_eq!(resolve(None, &s, &OK, &c), Verdict::Ask { pool: vec![7, 8], reasons: vec![AskReason::Weak] });
    }

    #[test]
    fn twin_rule_can_be_switched_off() {
        let c = cfg().with_ask(20, false, 0.40);
        let s = scored(&[(7, 0.98, true), (8, 0.5, true)]);
        assert_eq!(resolve(None, &s, &Facts { has_twin: true, ..OK }, &c), Verdict::Silent { pool: 7, step: 5 });
    }

    #[test]
    fn step4_is_silent_unless_an_ask_only_rule_applies() {
        let c = cfg();
        let s = scored(&[(3, 0.2, true)]);
        assert_eq!(resolve(Some(3), &s, &OK, &c), Verdict::Silent { pool: 3, step: 4 });
        // a twin, or a short body, takes step 4 away: it falls to step 5 and asks
        assert!(matches!(resolve(Some(3), &s, &Facts { has_twin: true, ..OK }, &c), Verdict::Ask { .. } | Verdict::Nothing));
        assert!(matches!(resolve(Some(3), &s, &Facts { lost_tokens: 5, ..OK }, &c), Verdict::Ask { .. } | Verdict::Nothing));
    }
}
