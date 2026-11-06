//! S4 / VS-O10: tune the ladder's tiers on a labelled set built from real functions of open-source repos.
//! Tune on one group of repos, validate on another, so a setting does not just fit the data it was picked on.
//!
//!   S4_SCRATCH=<folder with httpx, requests, flask> S4_LIGHTRAG=<repo> cargo run --release --bin tune -- [flags]
//!   flags: --ask=MIN,on|off,FLOOR (the twin rule)   --provisional-accuracy=0.95   --diag
//!
//! Layout:  dataset.rs (the labelled set)  simulate.rs (a case becomes a before/after pair)  counts.rs (right/wrong tallies)
//!          silent.rs (the silent rule)    provisional.rs (the middle tier)                  report.rs (markdown sections)

mod counts;
mod dataset;
mod provisional;
mod report;
mod silent;
mod simulate;

use std::fmt::Write as _;
use std::path::PathBuf;

use spike_match::cli;
use spike_match::driver::Driver;
use spike_match::ladder::{Config, Main, Steps, Verdict};

use counts::{Counts, classify, decide, run};
use dataset::Repo;
use simulate::Prepared;

fn main() -> anyhow::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let data = root.join("data");
    let scratch = PathBuf::from(std::env::var("S4_SCRATCH").expect("set S4_SCRATCH to the folder holding httpx/requests/flask"));
    let lightrag = PathBuf::from(std::env::var("S4_LIGHTRAG").expect("set S4_LIGHTRAG"));
    let mut driver = Driver::start(&root.join("../s2-python-driver/spike-driver-python/target/release/spike-driver-python"))?;

    // The base config: only the ask rules (twin rule, ask floor) come from here; the grid chooses the rest.
    let base = cli::apply(Config::default().with_steps(Steps::all()).with_ask(20, true, 0.30));
    let min_accuracy: f32 = cli::value("provisional-accuracy").and_then(|v| v.parse().ok()).unwrap_or(0.95);

    let repo = |name: &'static str, path: PathBuf| Repo { name, path };
    let (mut train, mut test): (Vec<Prepared>, Vec<Prepared>) = (vec![], vec![]);
    for r in [repo("LightRAG", lightrag), repo("httpx", scratch.join("httpx"))] {
        train.extend(simulate::prepare_repo(&r, &data, &mut driver)?);
    }
    for r in [repo("requests", scratch.join("requests")), repo("flask", scratch.join("flask"))] {
        test.extend(simulate::prepare_repo(&r, &data, &mut driver)?);
    }

    if std::env::args().any(|a| a == "--diag") {
        return diag(&train, &base);
    }

    let mut md = String::new();
    writeln!(md, "# S4 tuning results\n\ntrain = LightRAG + httpx ({} cases), held-out = requests + flask ({} cases). ask floor {:.2}, twin rule {}.\n", train.len(), test.len(), base.ask.floor, if base.ask.twins { "on" } else { "off" })?;

    // 1. the silent rule, for both scores
    let mut best: Vec<(&str, Config)> = vec![];
    for (label, score) in [("Edit", Main::Edit), ("Jaccard", Main::Jaccard)] {
        let ok = silent::search(&train, &base, score);
        silent::report(&mut md, label, &ok)?;
        if let Some((cfg, _)) = ok.into_iter().next() {
            best.push((label, cfg));
        }
    }

    // 2. validate the winners on repos they were not tuned on, and show how close the cliff is
    writeln!(md, "## Held-out check (requests + flask)\n")?;
    writeln!(md, "| score | threshold | margin | min tokens | silent right | **silent WRONG** | twin cases (silently wrong) | asked | missed | wrong silent if threshold −0.02 / −0.05 / −0.10 (train+test) |\n|---|---|---|---|---|---|---|---|---|---|")?;
    for (label, cfg) in &best {
        let c = run(&test, cfg);
        let cliff = report::cliff(cfg, &train, &test);
        writeln!(md, "| {label} | {:.2} | {:.2} | {} | {} | **{}** | {} ({}) | {} | {} | {} / {} / {} |", cfg.silent.threshold, cfg.silent.margin, cfg.ask.min_tokens, c.silent_right, c.silent_wrong, c.twin_cases, c.twin_silent_wrong, c.asked(), c.missed, cliff[0], cliff[1], cliff[2])?;
    }

    // 3. breakdowns and features
    let all: Vec<&Prepared> = train.iter().chain(test.iter()).collect();
    for (label, cfg) in &best {
        report::breakdown(&mut md, label, cfg, &all)?;
    }
    report::features(&mut md, &all)?;

    // 4. the provisional tier (Edit score)
    if let Some((_, silent_cfg)) = best.iter().find(|(l, _)| *l == "Edit") {
        provisional::report(&mut md, silent_cfg, &train, &test, min_accuracy)?;
    }

    print!("{md}");
    std::fs::write(data.join("tune-results.md"), &md)?;
    Ok(())
}

/// `--diag`: list the wrong silent matches at the strictest setting (what to look at when no setting is safe).
fn diag(train: &[Prepared], base: &Config) -> anyhow::Result<()> {
    let strict = base.clone().with_score(Main::Edit).with_silent(0.99, 0.30).with_ask(50, base.ask.twins, base.ask.floor);
    let mut shown = 0;
    for p in train {
        let mut c = Counts::default();
        classify(p, &strict, &mut c);
        if c.silent_wrong + c.twin_silent_wrong > 0 && shown < 12 {
            shown += 1;
            if let Verdict::Silent { pool, step } = decide(p, &strict) {
                eprintln!("WRONG SILENT {:?} level {} step {step} (twin case: {}): lost {} -> chose {} (truth {:?})", p.place, p.level, p.facts.has_twin, p.lost_q, p.pool_q[pool], p.truth.map(|t| p.pool_q[t].clone()));
            }
        }
    }
    Ok(())
}
