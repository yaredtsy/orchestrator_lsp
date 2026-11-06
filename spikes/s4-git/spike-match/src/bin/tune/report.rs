//! Markdown sections that describe a chosen configuration.

use std::fmt::Write as _;

use spike_match::ladder::Config;

use crate::counts::{Counts, classify, run};
use crate::simulate::{Place, Prepared};

const LEVELS: [f32; 7] = [0.0, 0.05, 0.10, 0.20, 0.35, 0.50, 0.70];

/// Train and held-out together, by edit level and placement.
pub fn breakdown(md: &mut String, label: &str, cfg: &Config, all: &[&Prepared]) -> std::fmt::Result {
    writeln!(md, "\n## Breakdown, {label} score (threshold {:.2}, margin {:.2}, min tokens {}), train + held-out\n", cfg.silent.threshold, cfg.silent.margin, cfg.ask.min_tokens)?;
    writeln!(md, "| edit level | placement | cases | silent right | asked | missed | wrong |\n|---|---|---|---|---|---|---|")?;
    for level in LEVELS {
        for place in [Place::SameScope, Place::ModuleLevel, Place::MovedFile] {
            let mut c = Counts::default();
            for p in all.iter().filter(|p| p.place == place && (p.level - level).abs() < 1e-3) {
                classify(p, cfg, &mut c);
            }
            if c.total() == 0 {
                continue;
            }
            writeln!(md, "| {:.0}% | {:?} | {} | {} | {} | {} | {} |", level * 100.0, place, c.total(), c.silent_right, c.asked(), c.missed, c.silent_wrong)?;
        }
    }
    let mut c = Counts::default();
    for p in all.iter().filter(|p| p.place == Place::Deleted) {
        classify(p, cfg, &mut c);
    }
    writeln!(md, "| (deleted) | Deleted | {} | silent {} | asked {} | – | – |", c.total(), c.silent_wrong, c.deleted_asked)
}

/// The true successor against the best WRONG candidate, by edit level (same-scope placement, mean over cases).
pub fn features(md: &mut String, all: &[&Prepared]) -> std::fmt::Result {
    writeln!(md, "\n## Features: the true successor vs the best WRONG candidate (same-scope placement, mean over cases)\n")?;
    writeln!(md, "| edit level | true: jaccard | true: edit | true: name | true: params | best wrong: jaccard | best wrong: edit | best wrong: name |\n|---|---|---|---|---|---|---|---|")?;
    for level in LEVELS {
        let (mut n, mut tj, mut te, mut tn, mut tp, mut wj, mut we, mut wn) = (0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32);
        for p in all.iter().filter(|p| p.place == Place::SameScope && (p.level - level).abs() < 1e-3) {
            let truth = p.truth.unwrap();
            let Some(t) = p.scored.iter().find(|s| s.idx == truth) else { continue };
            let Some(w) = p.scored.iter().filter(|s| s.idx != truth).max_by(|a, b| a.f.edit.total_cmp(&b.f.edit)) else { continue };
            n += 1.0;
            tj += t.f.jaccard;
            te += t.f.edit;
            tn += t.f.name;
            tp += t.f.params;
            wj += w.f.jaccard;
            we += w.f.edit;
            wn += w.f.name;
        }
        if n > 0.0 {
            writeln!(md, "| {:.0}% | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |", level * 100.0, tj / n, te / n, tn / n, tp / n, wj / n, we / n, wn / n)?;
        }
    }
    Ok(())
}

/// How far below the chosen threshold the wrong silent matches start (the cliff).
pub fn cliff(cfg: &Config, train: &[Prepared], test: &[Prepared]) -> Vec<u32> {
    [0.02, 0.05, 0.10]
        .iter()
        .map(|d| {
            let lo = cfg.clone().with_silent(cfg.silent.threshold - d, cfg.silent.margin);
            run(train, &lo).silent_wrong + run(test, &lo).silent_wrong
        })
        .collect()
}
