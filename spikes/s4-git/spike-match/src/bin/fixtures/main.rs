//! S4: refactor fixtures on a small Python repo, one git branch each, off a common base. For every documented node it
//! prints how the ladder resolved it and compares that with the known truth.
//!   cargo run --release --bin fixtures -- [--score=..] [--silent=T,M] [--provisional=F,M] [--ask=MIN,on|off,FLOOR]

mod cases;
mod repo;

use std::path::{Path, PathBuf};

use spike_match::cli;
use spike_match::driver::Driver;
use spike_match::git::{CliGit, GitBackend};
use spike_match::ladder::{Config, Main, Outcome, locate, matching};
use spike_match::model::Node;
use spike_match::snapshot::snapshot;

#[derive(Default)]
struct Tally {
    ok: u32,
    asked: u32,
    provisional_right: u32,
    provisional_wrong: u32,
    missed: u32,
    wrong_silent: u32,
}

/// Compares one outcome with the truth; returns the verdict text and updates the tally.
fn judge(out: &Outcome, want: &Option<Node>, t: &mut Tally) -> &'static str {
    match (out, want) {
        (Outcome::Found { node: got, .. }, Some(w)) if got == w => {
            t.ok += 1;
            "ok"
        }
        (Outcome::Found { .. }, _) => {
            t.wrong_silent += 1;
            "**WRONG SILENT MATCH**"
        }
        (Outcome::Provisional(c), Some(w)) if c.label() == w.to_string() => {
            t.provisional_right += 1;
            "provisional, right"
        }
        (Outcome::Provisional(_), _) => {
            t.provisional_wrong += 1;
            "**provisional, WRONG**"
        }
        (Outcome::Ask { candidates, .. }, Some(w)) => {
            t.asked += 1;
            if candidates[0].label() == w.to_string() { "asked, top candidate is right" } else { "asked, top candidate is WRONG" }
        }
        (Outcome::Ask { .. }, None) => {
            t.asked += 1;
            "asked (it was deleted)"
        }
        (Outcome::Missing, None) => {
            t.ok += 1;
            "ok (Missing)"
        }
        (Outcome::Missing, Some(_)) => {
            t.missed += 1;
            "MISSED"
        }
    }
}

fn main() -> anyhow::Result<()> {
    let tmp: PathBuf = std::env::temp_dir().join(format!("vnoc-s4-fixtures-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    repo::base(&tmp);
    let nodes = cases::nodes();
    let cases = cases::cases();
    for c in &cases {
        repo::branch(&tmp, c.branch, &*c.edit);
    }

    let driver_bin = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../s2-python-driver/spike-driver-python/target/release/spike-driver-python");
    let mut driver = Driver::start(&driver_bin)?;
    let g = CliGit::new(&tmp);
    let before_c = g.rev_parse("base")?;
    let before = snapshot(&g, &mut driver, &before_c)?;

    let scores = if cli::value("score").is_some() { vec![cli::apply(Config::default()).score] } else { vec![Main::Edit, Main::Jaccard] };
    for score in scores {
        let cfg = cli::apply(Config::default().with_score(score));
        println!("\n## ladder, score = {score:?}, silent {:?}, provisional {:?}, ask {:?}\n", cfg.silent, cfg.provisional, cfg.ask);
        println!("| fixture | node | result | verdict |\n|---|---|---|---|");
        let mut t = Tally::default();
        for case in &cases {
            let after_c = g.rev_parse(case.branch)?;
            let after = snapshot(&g, &mut driver, &after_c)?;
            let renames = g.renames(&before_c, &after_c)?;
            let m = matching(&before, &after, &renames);
            for (node, want) in nodes.iter().zip(case.truth(&nodes)) {
                let out = locate(node, &before, &after, &renames, &m, &cfg);
                let verdict = judge(&out, &want, &mut t);
                // unchanged nodes are step 1 and uninteresting
                if !matches!(&out, Outcome::Found { step: 1, .. }) {
                    println!("| {} {} | {node} | {} | {verdict} |", case.branch, case.desc, out.short());
                }
            }
        }
        println!(
            "\nok {} · asked {} · provisional {} right / {} wrong · missed {} · **wrong silent {}**",
            t.ok, t.asked, t.provisional_right, t.provisional_wrong, t.missed, t.wrong_silent
        );
    }
    std::fs::remove_dir_all(&tmp).ok();
    Ok(())
}
