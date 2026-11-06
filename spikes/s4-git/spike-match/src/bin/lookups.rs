//! Times the three VS-D74 lookups with the git binary and with gix, on a repo you give it.
//!   cargo run --release --bin lookups -- <repo> <path that is in HEAD> [<path that is not>]

use std::time::Instant;

use spike_match::git::{BranchLookups, CliGit, GixGit};

fn time<T>(n: usize, f: impl Fn() -> T) -> (T, f64) {
    let mut us: Vec<f64> = Vec::with_capacity(n);
    let mut last = None;
    for _ in 0..n {
        let t = Instant::now();
        last = Some(f());
        us.push(t.elapsed().as_secs_f64() * 1e6);
    }
    us.sort_by(|a, b| a.total_cmp(b));
    (last.unwrap(), us[n / 2])
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (repo, present, absent) = (&a[0], &a[1], a.get(2).map_or("definitely/not/here.py", String::as_str));
    let backends: Vec<Box<dyn BranchLookups>> = vec![Box::new(CliGit::new(repo)), Box::new(GixGit::new(repo))];
    println!("repo: {repo}\n");
    println!("| lookup | {} | {} |\n|---|---|---|", backends[0].name(), backends[1].name());
    type Row<'a> = (&'a str, Box<dyn Fn(&dyn BranchLookups) -> String + 'a>);
    let rows: Vec<Row> = vec![
        ("is_repo", Box::new(|g| g.is_repo().to_string())),
        ("path_in_head (present)", Box::new(|g| g.path_in_head(present).unwrap().to_string())),
        ("path_in_head (absent)", Box::new(|g| g.path_in_head(absent).unwrap().to_string())),
        ("current_branch", Box::new(|g| format!("{:?}", g.current_branch().unwrap()))),
        ("local_branches", Box::new(|g| format!("{:?}", g.local_branches().unwrap()))),
    ];
    for (name, f) in &rows {
        let cells: Vec<(String, f64)> = backends.iter().map(|b| time(30, || f(b.as_ref()))).collect();
        let agree = cells[0].0 == cells[1].0;
        let verdict = if agree { String::new() } else { format!(" **DISAGREE, gix says {}**", cells[1].0) };
        println!("| {name} -> {}{verdict} | {:.0} µs | {:.0} µs |", cells[0].0, cells[0].1, cells[1].1);
    }
}
