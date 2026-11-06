//! The `git` binary behind `GitBackend`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, anyhow};

use super::{GitBackend, Rename};

pub struct CliGit {
    pub dir: PathBuf,
}

impl CliGit {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        CliGit { dir: dir.as_ref().to_path_buf() }
    }
    fn run(&self, args: &[&str]) -> anyhow::Result<std::process::Output> {
        Command::new("git").arg("-C").arg(&self.dir).args(args).output().with_context(|| format!("git {args:?}"))
    }
    fn ok_text(&self, args: &[&str]) -> anyhow::Result<String> {
        let o = self.run(args)?;
        if !o.status.success() {
            return Err(anyhow!("git {args:?}: {}", String::from_utf8_lossy(&o.stderr)));
        }
        Ok(String::from_utf8_lossy(&o.stdout).trim_end().to_string())
    }
}

impl GitBackend for CliGit {
    fn name(&self) -> &'static str {
        "git (cli)"
    }
    fn rev_parse(&self, rev: &str) -> anyhow::Result<String> {
        self.ok_text(&["rev-parse", rev])
    }
    fn list_py_files(&self, commit: &str) -> anyhow::Result<Vec<String>> {
        let t = self.ok_text(&["ls-tree", "-r", "--name-only", commit])?;
        Ok(t.lines().filter(|l| l.ends_with(".py")).map(String::from).collect())
    }
    fn read_file(&self, commit: &str, path: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let o = self.run(&["show", &format!("{commit}:{path}")])?;
        Ok(o.status.success().then_some(o.stdout))
    }
    fn renames(&self, before: &str, after: &str) -> anyhow::Result<Vec<Rename>> {
        let t = self.ok_text(&["diff", "-M", "--name-status", before, after])?;
        Ok(t.lines()
            .filter_map(|l| {
                let mut p = l.split('\t');
                let st = p.next()?;
                let score: u32 = st.strip_prefix('R')?.parse().ok()?;
                Some(Rename { old: p.next()?.to_string(), new: p.next()?.to_string(), score })
            })
            .collect())
    }
    fn is_ancestor(&self, old: &str, new: &str) -> anyhow::Result<bool> {
        let o = self.run(&["merge-base", "--is-ancestor", old, new])?;
        match o.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(anyhow!("merge-base: {}", String::from_utf8_lossy(&o.stderr))),
        }
    }
    fn has_commit(&self, rev: &str) -> anyhow::Result<bool> {
        Ok(self.run(&["cat-file", "-e", &format!("{rev}^{{commit}}")])?.status.success())
    }
    fn local_branch_tips(&self) -> anyhow::Result<Vec<(String, String)>> {
        let t = self.ok_text(&["for-each-ref", "--format=%(refname:short) %(objectname)", "refs/heads"])?;
        Ok(t.lines().filter_map(|l| l.split_once(' ').map(|(a, b)| (a.to_string(), b.to_string()))).collect())
    }
    fn last_commit_touching(&self, rev: &str, path: &str) -> anyhow::Result<Option<String>> {
        let t = self.ok_text(&["log", "-1", "--format=%H", rev, "--", path])?;
        Ok((!t.is_empty()).then_some(t))
    }
}
