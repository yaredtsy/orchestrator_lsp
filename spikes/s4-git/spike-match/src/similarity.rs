//! VS-O10: two body scores, and the context features that sit next to them.
//!
//! Body scores (both 0.0..=1.0, 1.0 = identical token streams):
//!   * `jaccard`: Jaccard similarity of the sets of token 3-grams. Order-insensitive at long range, fast.
//!   * `edit`:    1 - (token-level Levenshtein distance / the longer length). Order-sensitive, O(n*m).
//! Context features (reported next to the scores, and used by the silent-match rule):
//!   same_file, same_class, parameter-name overlap, name similarity.

use std::collections::HashSet;

use crate::model::Def;

fn hash64(s: &str) -> u64 {
    let b = blake3::hash(s.as_bytes());
    u64::from_le_bytes(b.as_bytes()[..8].try_into().unwrap())
}

/// A def with its token hashes and 3-gram set precomputed (a pool is scored against many lost defs).
pub struct Fp<'a> {
    pub def: &'a Def,
    pub toks: Vec<u64>,
    pub grams: HashSet<u64>,
    pub words: HashSet<String>,
    pub params: HashSet<&'a str>,
}

/// snake_case and CamelCase -> a set of lower-case words.
pub fn words(name: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c == '_' {
            if !cur.is_empty() {
                out.insert(std::mem::take(&mut cur));
            }
            prev_lower = false;
        } else if c.is_uppercase() && prev_lower {
            out.insert(std::mem::take(&mut cur));
            cur.extend(c.to_lowercase());
            prev_lower = false;
        } else {
            cur.extend(c.to_lowercase());
            prev_lower = c.is_lowercase();
        }
    }
    if !cur.is_empty() {
        out.insert(cur);
    }
    out
}

impl<'a> Fp<'a> {
    pub fn new(def: &'a Def) -> Self {
        let toks: Vec<u64> = def.tokens.iter().map(|t| hash64(t)).collect();
        let grams = toks
            .windows(3)
            .map(|w| w[0].wrapping_mul(1_000_003).wrapping_add(w[1]).wrapping_mul(1_000_003).wrapping_add(w[2]))
            .collect();
        Fp { def, toks, grams, words: words(&def.name), params: def.params.iter().map(String::as_str).collect() }
    }
}

fn jaccard_set<T: std::hash::Hash + Eq>(a: &HashSet<T>, b: &HashSet<T>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = (a.len() + b.len()) as f32 - inter;
    inter / union
}

pub fn jaccard(a: &Fp, b: &Fp) -> f32 {
    jaccard_set(&a.grams, &b.grams)
}

/// Token-level normalized edit similarity.
pub fn edit(a: &Fp, b: &Fp) -> f32 {
    let (x, y) = (&a.toks, &b.toks);
    let longest = x.len().max(y.len());
    if longest == 0 {
        return 1.0;
    }
    let mut prev: Vec<u32> = (0..=y.len() as u32).collect();
    let mut cur = vec![0u32; y.len() + 1];
    for (i, xi) in x.iter().enumerate() {
        cur[0] = i as u32 + 1;
        for (j, yj) in y.iter().enumerate() {
            let sub = prev[j] + u32::from(xi != yj);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    1.0 - prev[y.len()] as f32 / longest as f32
}

pub fn name_sim(a: &Fp, b: &Fp) -> f32 {
    jaccard_set(&a.words, &b.words)
}

pub fn param_sim(a: &Fp, b: &Fp) -> f32 {
    jaccard_set(&a.params, &b.params)
}

/// Everything reported for one (lost, candidate) pair.
#[derive(Debug, Clone)]
pub struct Features {
    pub jaccard: f32,
    pub edit: f32,
    pub name: f32,
    pub params: f32,
    pub same_file: bool,
    pub same_class: bool,
}

/// `mapped_file`: where the lost def's file is now (it may have been renamed/moved by ladder step 2 or 3).
pub fn features(lost: &Fp, cand: &Fp, mapped_file: &str) -> Features {
    Features {
        jaccard: jaccard(lost, cand),
        edit: edit(lost, cand),
        name: name_sim(lost, cand),
        params: param_sim(lost, cand),
        same_file: cand.def.file == mapped_file,
        // "same class" = both are methods of a class with the same name and qualified parent, in the same file
        same_class: lost.def.parent_kind.as_deref() == Some("class")
            && cand.def.parent_kind.as_deref() == Some("class")
            && cand.def.parent == lost.def.parent
            && cand.def.file == mapped_file,
    }
}
