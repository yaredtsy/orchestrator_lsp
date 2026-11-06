//! VS-O10: search for the silent rule. "Zero wrong silent matches" is the constraint; among the settings that meet it
//! the winner is the one with the most right silent matches, then the most conservative one.

use std::fmt::Write as _;

use spike_match::ladder::{Config, Main};

use crate::counts::{Counts, run};
use crate::simulate::Prepared;

pub const THRESHOLDS: std::ops::RangeInclusive<u32> = 40..=99;
pub const MARGINS: [f32; 6] = [0.0, 0.05, 0.10, 0.15, 0.20, 0.30];
pub const MIN_TOKENS: [usize; 4] = [10, 20, 30, 50];

pub fn grid_size() -> usize {
    THRESHOLDS.clone().count() * MARGINS.len() * MIN_TOKENS.len()
}

/// Every grid point with zero wrong silent matches on `train` (twin cases are judged apart), best first.
pub fn search(train: &[Prepared], base: &Config, score: Main) -> Vec<(Config, Counts)> {
    let mut ok = vec![];
    for t in THRESHOLDS.map(|x| x as f32 / 100.0) {
        for margin in MARGINS {
            for min_tokens in MIN_TOKENS {
                let cfg = base.clone().with_score(score).with_silent(t, margin).with_ask(min_tokens, base.ask.twins, base.ask.floor);
                let c = run(train, &cfg);
                if c.silent_wrong == 0 {
                    ok.push((cfg, c));
                }
            }
        }
    }
    // most silent-right first; ties -> the most conservative (highest threshold, then margin, then min_tokens)
    ok.sort_by(|a, b| {
        b.1.silent_right
            .cmp(&a.1.silent_right)
            .then(b.0.silent.threshold.total_cmp(&a.0.silent.threshold))
            .then(b.0.silent.margin.total_cmp(&a.0.silent.margin))
            .then(b.0.ask.min_tokens.cmp(&a.0.ask.min_tokens))
    });
    ok
}

pub fn report(md: &mut String, label: &str, ok: &[(Config, Counts)]) -> std::fmt::Result {
    writeln!(md, "## {label} score: configs with ZERO wrong silent matches on the training set (twin cases judged apart): {} of {}\n", ok.len(), grid_size())?;
    writeln!(md, "| threshold | margin | min tokens | silent right | asked (top right) | asked (in list) | missed | deleted asked | twin cases | **twin cases silently WRONG** |\n|---|---|---|---|---|---|---|---|---|---|")?;
    for (cfg, c) in ok.iter().take(5) {
        writeln!(
            md,
            "| {:.2} | {:.2} | {} | {} | {} | {} | {} | {} | {} | **{}** |",
            cfg.silent.threshold, cfg.silent.margin, cfg.ask.min_tokens, c.silent_right, c.ask_top_right, c.ask_in_list, c.missed, c.deleted_asked, c.twin_cases, c.twin_silent_wrong
        )?;
    }
    writeln!(md)
}
