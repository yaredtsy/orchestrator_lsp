//! Flags shared by the binaries, so a ladder setting is always spelled the same way:
//!   --score=edit|jaccard   --silent=THRESHOLD,MARGIN   --provisional=FLOOR,MARGIN   --ask=MIN_TOKENS,on|off,FLOOR
//! (`--ask`'s `on|off` is the twin rule.)

use crate::ladder::{Config, Main};

/// The value of `--name=value`, if given.
pub fn value(name: &str) -> Option<String> {
    let prefix = format!("--{name}=");
    std::env::args().find_map(|a| a.strip_prefix(&prefix).map(String::from))
}

fn nums(v: &str) -> Vec<f32> {
    v.split(',').filter_map(|x| x.trim().parse().ok()).collect()
}

/// Applies whatever flags were given on top of `base`.
pub fn apply(mut cfg: Config) -> Config {
    if let Some(v) = value("score") {
        cfg.score = if v == "jaccard" { Main::Jaccard } else { Main::Edit };
    }
    if let Some(v) = value("silent") {
        if let [t, m] = nums(&v)[..] {
            cfg = cfg.with_silent(t, m);
        }
    }
    if let Some(v) = value("provisional") {
        if let [f, m] = nums(&v)[..] {
            cfg = cfg.with_provisional(f, m);
        }
    }
    if let Some(v) = value("ask") {
        let p: Vec<&str> = v.split(',').collect();
        if let [min, twins, floor] = p[..] {
            cfg = cfg.with_ask(min.parse().unwrap_or(20), twins != "off", floor.parse().unwrap_or(0.5));
        }
    }
    cfg
}
