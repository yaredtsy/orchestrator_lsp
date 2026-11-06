//! Scoring the candidates. Scoring is the expensive part and does not depend on the thresholds, so the tuning harness
//! scores once and then asks `policy::resolve` about many different `Config`s.

use crate::model::Def;
use crate::similarity::{Features, Fp, features, jaccard, name_sim};

use super::config::Main;

/// One scored candidate. `idx` is the candidate's position in the pool it was drawn from.
#[derive(Debug, Clone)]
pub struct Scored {
    pub idx: usize,
    pub f: Features,
}

/// Scores the most promising candidates: the 12 best by Jaccard plus the 4 whose names are closest.
/// (Edit distance is O(n*m), so it is only computed for these 16.)
pub fn score_pool(lost_def: &Def, pool: &[&Def], mapped_file: &str) -> Vec<Scored> {
    let lost = Fp::new(lost_def);
    let fps: Vec<Fp> = pool.iter().map(|d| Fp::new(d)).collect();
    let mut by_jac: Vec<(usize, f32)> = fps.iter().enumerate().map(|(i, c)| (i, jaccard(&lost, c))).collect();
    by_jac.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut keep: Vec<usize> = by_jac.iter().take(12).map(|x| x.0).collect();
    let mut by_name: Vec<(usize, f32)> = fps.iter().enumerate().map(|(i, c)| (i, name_sim(&lost, c))).collect();
    by_name.sort_by(|a, b| b.1.total_cmp(&a.1));
    keep.extend(by_name.iter().take(4).map(|x| x.0));
    keep.sort();
    keep.dedup();
    keep.into_iter().map(|i| Scored { idx: i, f: features(&lost, &fps[i], mapped_file) }).collect()
}

/// Positions in `scored`, best first: by the main score, ties broken by name similarity (so an asked list puts the likelier one first).
pub fn order(scored: &[Scored], main: Main) -> Vec<usize> {
    let mut o: Vec<usize> = (0..scored.len()).collect();
    o.sort_by(|&a, &b| main.of(&scored[b].f).total_cmp(&main.of(&scored[a].f)).then(scored[b].f.name.total_cmp(&scored[a].f.name)));
    o
}

/// The best candidate, its score, and its lead over the second best (the second best of one candidate scores 0).
#[derive(Debug, Clone, Copy)]
pub struct Lead {
    /// position in `scored`
    pub pos: usize,
    pub best: f32,
    pub margin: f32,
}

pub fn lead(scored: &[Scored], main: Main) -> Option<Lead> {
    let o = order(scored, main);
    let &pos = o.first()?;
    let best = main.of(&scored[pos].f);
    let second = o.get(1).map_or(0.0, |&i| main.of(&scored[i].f));
    Some(Lead { pos, best, margin: best - second })
}
