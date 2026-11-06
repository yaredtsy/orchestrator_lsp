//! The provisional tier (between a silent match and a question): where to put its floor.
//!
//! Method. Take the cases that are still a QUESTION under the chosen silent rule for a soft reason (the score or the
//! lead is not good enough: `Ambiguous` / `Weak`), not because an ask-only rule applies (twin, short body).
//! For each, look at the top candidate: its Edit score, its lead over the second, and whether it is the real successor.
//! A provisional link is only worth showing if the top candidate is right nearly always, so the floor goes where
//! top-1 accuracy is at least `min_accuracy` (with a margin over the second). Tune on train, check on held-out.

use std::fmt::Write as _;

use spike_match::ladder::{AskReason, Config, Verdict, lead};

use crate::counts::{Counts, decide, run};
use crate::simulate::Prepared;

/// A case that is a soft question: the best candidate was not linked, but could be.
pub struct SoftAsk {
    pub best: f32,
    pub margin: f32,
    /// the top candidate is the real successor
    pub right: bool,
    /// the function was in fact deleted (so any link is wrong)
    pub deleted: bool,
    pub level: f32,
}

/// `cfg` must have the provisional tier OFF (so a soft question is still a question).
pub fn soft_asks(set: &[Prepared], cfg: &Config) -> Vec<SoftAsk> {
    set.iter()
        .filter_map(|p| {
            let Verdict::Ask { reasons, .. } = decide(p, cfg) else { return None };
            if reasons.iter().any(|r| matches!(r, AskReason::Twin | AskReason::ShortBody)) {
                return None;
            }
            let l = lead(&p.scored, cfg.score)?;
            Some(SoftAsk { best: l.best, margin: l.margin, right: p.truth == Some(p.scored[l.pos].idx), deleted: p.truth.is_none(), level: p.level })
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct Pick {
    pub floor: f32,
    pub margin: f32,
}

fn accuracy(set: &[SoftAsk], floor: f32, margin: f32) -> (usize, usize) {
    let sel: Vec<&SoftAsk> = set.iter().filter(|s| s.best >= floor && s.margin >= margin).collect();
    (sel.len(), sel.iter().filter(|s| s.right).count())
}

pub const MARGINS: [f32; 5] = [0.05, 0.10, 0.15, 0.20, 0.30];
/// a setting that selects fewer cases than this is too small to trust
pub const MIN_CASES: usize = 30;

/// The setting with the most provisional links whose top-1 accuracy on `train` is at least `min_accuracy`.
pub fn select(train: &[SoftAsk], min_accuracy: f32, ceiling: f32) -> Option<Pick> {
    let mut best: Option<(usize, Pick)> = None;
    for floor in (30..(ceiling * 100.0) as u32).map(|x| x as f32 / 100.0) {
        for margin in MARGINS {
            let (n, right) = accuracy(train, floor, margin);
            if n < MIN_CASES || (right as f32 / n as f32) < min_accuracy {
                continue;
            }
            // most links first; ties -> the higher floor, then the larger margin
            let better = match &best {
                None => true,
                Some((bn, bp)) => n > *bn || (n == *bn && (floor > bp.floor || (floor == bp.floor && margin > bp.margin))),
            };
            if better {
                best = Some((n, Pick { floor, margin }));
            }
        }
    }
    best.map(|b| b.1)
}

fn band_table(md: &mut String, title: &str, train: &[SoftAsk], test: &[SoftAsk], margin: f32) -> std::fmt::Result {
    writeln!(md, "\n{title}\n")?;
    writeln!(md, "| Edit-score band | train: cases | train: top-1 right | train: accuracy | held-out: cases | held-out: top-1 right | held-out: accuracy |\n|---|---|---|---|---|---|---|")?;
    let mut lo = 0.30f32;
    while lo < 0.999 {
        let hi = (lo + 0.05).min(1.0001);
        let in_band = |s: &&SoftAsk| s.best >= lo && s.best < hi && s.margin >= margin;
        let row = |set: &[SoftAsk]| {
            let b: Vec<&SoftAsk> = set.iter().filter(in_band).collect();
            (b.len(), b.iter().filter(|s| s.right).count())
        };
        let ((tn, tr), (hn, hr)) = (row(train), row(test));
        let pct = |r: usize, n: usize| if n == 0 { "–".to_string() } else { format!("{:.0}%", 100.0 * r as f32 / n as f32) };
        if tn + hn > 0 {
            writeln!(md, "| {:.2} – {:.2} | {tn} | {tr} | {} | {hn} | {hr} | {} |", lo, hi.min(1.0), pct(tr, tn), pct(hr, hn))?;
        }
        lo += 0.05;
    }
    Ok(())
}

pub fn report(md: &mut String, silent: &Config, train: &[Prepared], test: &[Prepared], min_accuracy: f32) -> std::fmt::Result {
    let off = Config { provisional: None, ..silent.clone() };
    let (tr_soft, te_soft) = (soft_asks(train, &off), soft_asks(test, &off));
    writeln!(md, "\n# The provisional tier (Edit score)\n")?;
    writeln!(md, "Silent rule used: threshold {:.2}, margin {:.2}, min tokens {}, twin rule {}. Cases that are still a soft question under it (not forced by a twin or a short body): {} train, {} held-out.",
        silent.silent.threshold, silent.silent.margin, silent.ask.min_tokens, if silent.ask.twins { "on" } else { "off" }, tr_soft.len(), te_soft.len())?;

    band_table(md, "### Top-1 accuracy of the best candidate, per Edit-score band (any lead)", &tr_soft, &te_soft, 0.0)?;
    band_table(md, "### The same, only where the lead over the second best is at least 0.10", &tr_soft, &te_soft, 0.10)?;

    accuracy_targets(md, &tr_soft, &te_soft, silent.silent.threshold)?;
    wrong_composition(md, &tr_soft, &te_soft)?;

    // the requested target, then the reachable fallbacks, each with what it would move
    let mut targets = vec![min_accuracy];
    targets.extend([0.90, 0.85, 0.80].into_iter().filter(|t| *t < min_accuracy));
    let picks: Vec<(f32, Pick)> = targets.iter().filter_map(|&t| select(&tr_soft, t, silent.silent.threshold).map(|p| (t, p))).collect();
    match picks.first() {
        Some((t, pick)) if *t == min_accuracy => writeln!(md, "\n### Chosen provisional floor for {:.0}%: Edit score >= {:.2} with a lead >= {:.2}", t * 100.0, pick.floor, pick.margin)?,
        _ => writeln!(md, "\n### No (floor, margin) reaches {:.0}% top-1 accuracy with at least {MIN_CASES} cases on the training set. The reachable targets and what they would move:", min_accuracy * 100.0)?,
    }

    // what each tier setting changes in the whole run: cases that were questions and are now linked
    let off = off.clone();
    writeln!(md, "\n### What moves from ask to provisional\n")?;
    writeln!(md, "| target accuracy | floor / lead | set | cases | asked before | asked after | **moved to provisional** | provisional right | provisional wrong | (of those: for a deleted function) | silent right (unchanged) |\n|---|---|---|---|---|---|---|---|---|---|---|")?;
    for (t, pick) in &picks {
        let on = silent.clone().with_provisional(pick.floor, pick.margin);
        for (name, set) in [("train", train), ("held-out", test)] {
            let (b, a): (Counts, Counts) = (run(set, &off), run(set, &on));
            writeln!(md, "| {:.0}% | {:.2} / {:.2} | {name} | {} | {} | {} | **{}** | {} | {} | {} | {} |", t * 100.0, pick.floor, pick.margin, b.total(), b.asked(), a.asked(), b.asked() - a.asked(), a.prov_right, a.prov_wrong, a.prov_on_deleted, a.silent_right)?;
        }
    }
    Ok(())
}

/// What can a (floor, margin) reach at different accuracy targets? (The chosen target may be out of reach; this shows what is.)
fn accuracy_targets(md: &mut String, train: &[SoftAsk], test: &[SoftAsk], ceiling: f32) -> std::fmt::Result {
    writeln!(md, "\n### What a floor can reach at different accuracy targets\n")?;
    writeln!(md, "| target top-1 accuracy | floor | lead | train: links | train: accuracy | held-out: links | held-out: accuracy |\n|---|---|---|---|---|---|---|")?;
    for target in [0.99, 0.95, 0.90, 0.85, 0.80] {
        match select(train, target, ceiling) {
            None => writeln!(md, "| {:.0}% | – | – | none reaches it | | | |", target * 100.0)?,
            Some(p) => {
                let ((tn, tr), (hn, hr)) = (accuracy(train, p.floor, p.margin), accuracy(test, p.floor, p.margin));
                let pct = |r: usize, n: usize| if n == 0 { "–".to_string() } else { format!("{:.1}%", 100.0 * r as f32 / n as f32) };
                writeln!(md, "| {:.0}% | {:.2} | {:.2} | {tn} | {} | {hn} | {} |", target * 100.0, p.floor, p.margin, pct(tr, tn), pct(hr, hn))?;
            }
        }
    }
    Ok(())
}

/// Where do the wrong top-1s come from? High-score, clear-lead cases only (where a link looks most tempting).
fn wrong_composition(md: &mut String, train: &[SoftAsk], test: &[SoftAsk]) -> std::fmt::Result {
    writeln!(md, "\n### Wrong top-1s among the tempting cases (Edit score >= 0.80, lead >= 0.10), by edit level\n")?;
    writeln!(md, "| set | edit level | cases | top-1 right | wrong: the function was DELETED | wrong: the real successor was outranked |\n|---|---|---|---|---|---|")?;
    for (name, set) in [("train", train), ("held-out", test)] {
        for (label, lo, hi) in [("deleted", -2.0f32, -0.5f32), ("0-5%", -0.01, 0.06), ("10-20%", 0.09, 0.21), ("35%+", 0.34, 1.0)] {
            let sel: Vec<&SoftAsk> = set.iter().filter(|s| s.best >= 0.80 && s.margin >= 0.10 && if label == "deleted" { s.deleted } else { !s.deleted && s.level > lo && s.level < hi }).collect();
            if sel.is_empty() {
                continue;
            }
            let right = sel.iter().filter(|s| s.right).count();
            let deleted = sel.iter().filter(|s| !s.right && s.deleted).count();
            writeln!(md, "| {name} | {label} | {} | {right} | {deleted} | {} |", sel.len(), sel.len() - right - deleted)?;
        }
    }
    Ok(())
}
