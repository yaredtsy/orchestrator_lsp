//! Classifying one decision against the truth, and the tallies.

use spike_match::ladder::{AskReason, Config, Verdict, resolve};

use crate::simulate::Prepared;

#[derive(Default, Clone, Debug)]
pub struct Counts {
    pub silent_right: u32,
    pub silent_wrong: u32,
    pub prov_right: u32,
    /// a provisional link to something that is not the successor (includes links made for deleted functions)
    pub prov_wrong: u32,
    /// of `prov_wrong`: links made for a function that was in fact deleted
    pub prov_on_deleted: u32,
    pub ask_top_right: u32,
    pub ask_in_list: u32,
    pub ask_no_truth: u32,
    pub missed: u32,
    pub deleted_asked: u32,
    pub deleted_ok: u32,
    /// asks forced by an ask-only rule (twin, short body), counted apart from the rest
    pub ask_only: u32,
    /// cases whose lost function has an identical twin (counted apart from the rest)
    pub twin_cases: u32,
    pub twin_silent_wrong: u32,
}

impl Counts {
    pub fn asked(&self) -> u32 {
        self.ask_top_right + self.ask_in_list + self.ask_no_truth + self.deleted_asked
    }
    pub fn total(&self) -> u32 {
        self.silent_right + self.silent_wrong + self.twin_silent_wrong + self.prov_right + self.prov_wrong + self.asked() + self.missed + self.deleted_ok
    }
}

pub fn decide(p: &Prepared, cfg: &Config) -> Verdict {
    resolve(p.exact_unique, &p.scored, &p.facts, cfg)
}

/// A wrong silent match in a TWIN case goes on its own line: the rest of the set is judged without it.
pub fn classify(p: &Prepared, cfg: &Config, c: &mut Counts) {
    let wrong_silent = |c: &mut Counts| if p.facts.has_twin { c.twin_silent_wrong += 1 } else { c.silent_wrong += 1 };
    if p.facts.has_twin {
        c.twin_cases += 1;
    }
    match (decide(p, cfg), p.truth) {
        (Verdict::Silent { pool, .. }, Some(t)) => {
            if pool == t { c.silent_right += 1 } else { wrong_silent(c) }
        }
        (Verdict::Silent { .. }, None) => wrong_silent(c),
        (Verdict::Provisional { pool }, Some(t)) => {
            if pool == t { c.prov_right += 1 } else { c.prov_wrong += 1 }
        }
        (Verdict::Provisional { .. }, None) => {
            c.prov_wrong += 1;
            c.prov_on_deleted += 1;
        }
        (Verdict::Ask { pool, reasons }, truth) => {
            if reasons.iter().any(|r| matches!(r, AskReason::Twin | AskReason::ShortBody)) {
                c.ask_only += 1;
            }
            match truth {
                Some(t) if pool.first() == Some(&t) => c.ask_top_right += 1,
                Some(t) if pool.contains(&t) => c.ask_in_list += 1,
                Some(_) => c.ask_no_truth += 1,
                None => c.deleted_asked += 1,
            }
        }
        (Verdict::Nothing, Some(_)) => c.missed += 1,
        (Verdict::Nothing, None) => c.deleted_ok += 1,
    }
}

pub fn run(set: &[Prepared], cfg: &Config) -> Counts {
    let mut c = Counts::default();
    for p in set {
        classify(p, cfg, &mut c);
    }
    c
}
