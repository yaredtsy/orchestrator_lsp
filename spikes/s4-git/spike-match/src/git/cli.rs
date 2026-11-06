//! The `git` binary behind `GitBackend`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, anyhow};

use super::{BranchLookups, GitBackend, Rename};

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

impl CliGit {
    /// The git directory (`.git`, or the real one for a worktree).
    pub fn git_dir(&self) -> anyhow::Result<PathBuf> {
        Ok(PathBuf::from(self.ok_text(&["rev-parse", "--absolute-git-dir"])?))
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
}

impl BranchLookups for CliGit {
    fn name(&self) -> &'static str {
        "git (cli)"
    }
    fn is_repo(&self) -> bool {
        self.run(&["rev-parse", "--git-dir"]).map(|o| o.status.success()).unwrap_or(false)
    }
    fn path_in_head(&self, path: &str) -> anyhow::Result<bool> {
        // `HEAD:path` fails both when the path is absent and when HEAD has no commit: both mean "not in HEAD's tree"
        Ok(self.run(&["cat-file", "-e", &format!("HEAD:{path}")])?.status.success())
    }
    fn current_branch(&self) -> anyhow::Result<Option<String>> {
        let o = self.run(&["symbolic-ref", "--short", "-q", "HEAD"])?;
        Ok(o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string()))
    }
    fn local_branches(&self) -> anyhow::Result<Vec<String>> {
        let mut v: Vec<String> = self.ok_text(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])?.lines().map(String::from).collect();
        v.sort();
        Ok(v)
    }
}
