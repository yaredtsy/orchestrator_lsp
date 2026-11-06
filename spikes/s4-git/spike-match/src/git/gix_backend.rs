//! The three VS-D74 lookups with `gix` (no process spawn).

use std::path::{Path, PathBuf};

use super::BranchLookups;

pub struct GixGit {
    dir: PathBuf,
}

impl GixGit {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        GixGit { dir: dir.as_ref().to_path_buf() }
    }
    fn open(&self) -> anyhow::Result<gix::Repository> {
        Ok(gix::open(&self.dir)?)
    }
}

impl BranchLookups for GixGit {
    fn name(&self) -> &'static str {
        "gix"
    }
    fn is_repo(&self) -> bool {
        gix::open(&self.dir).is_ok()
    }
    fn path_in_head(&self, path: &str) -> anyhow::Result<bool> {
        let repo = self.open()?;
        // an unborn branch has no commit and no tree
        let Ok(commit) = repo.head_commit() else { return Ok(false) };
        Ok(commit.tree()?.lookup_entry_by_path(path)?.is_some())
    }
    fn current_branch(&self) -> anyhow::Result<Option<String>> {
        Ok(self.open()?.head_name()?.map(|n| n.shorten().to_string()))
    }
    fn local_branches(&self) -> anyhow::Result<Vec<String>> {
        let repo = self.open()?;
        let mut v: Vec<String> = repo.references()?.local_branches()?.filter_map(Result::ok).map(|r| r.name().shorten().to_string()).collect();
        v.sort();
        Ok(v)
    }
}
