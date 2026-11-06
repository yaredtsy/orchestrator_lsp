//! Everything the matching and sync code needs from git, behind one trait.
//! `CliGit` (git/cli.rs) shells out to the `git` binary. A `gix` backend (S4 part 2) implements the same trait;
//! an operation that gix is too slow at, or lacks, falls back to `CliGit` and is recorded.

mod cli;

pub use cli::CliGit;

#[derive(Debug, Clone, PartialEq)]
pub struct Rename {
    pub old: String,
    pub new: String,
    /// git's own similarity, 0..=100
    pub score: u32,
}

pub trait GitBackend {
    fn name(&self) -> &'static str;
    fn rev_parse(&self, rev: &str) -> anyhow::Result<String>;
    fn list_py_files(&self, commit: &str) -> anyhow::Result<Vec<String>>;
    fn read_file(&self, commit: &str, path: &str) -> anyhow::Result<Option<Vec<u8>>>;
    /// Ladder step 3: file renames between two commits (git's rename detection, `-M`).
    fn renames(&self, before: &str, after: &str) -> anyhow::Result<Vec<Rename>>;
    /// Is `old` reachable from `new` by walking parents? (02-05 §5)
    fn is_ancestor(&self, old: &str, new: &str) -> anyhow::Result<bool>;
    /// Does the repo contain this object? (false in a shallow clone whose history was cut)
    fn has_commit(&self, rev: &str) -> anyhow::Result<bool>;
    fn local_branch_tips(&self) -> anyhow::Result<Vec<(String, String)>>;
    /// The last commit, reachable from `rev`, that changed `path`.
    fn last_commit_touching(&self, rev: &str, path: &str) -> anyhow::Result<Option<String>>;
}
