//! Everything the matching and sync code needs from git, behind two small traits.
//!
//! * `GitBackend`: what the ladder reads (commits, files, git's rename detection). The `git` binary does all of it (`cli.rs`).
//! * `BranchLookups`: the three lookups VS-D74 needs, and nothing else: is there a repository, is a path in HEAD's tree,
//!   which branch is HEAD on, which local branches exist. `CliGit` (`cli.rs`) and `GixGit` (`gix_backend.rs`) both implement it;
//!   an operation gix is too slow at, or lacks, falls back to the `git` binary through this same trait.
//!
//! No ancestry test and no history walk: VS-D74 dropped them.

mod cli;
mod gix_backend;

pub use cli::CliGit;
pub use gix_backend::GixGit;

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
}

/// The three lookups of VS-D74. All of them work on a shallow clone, an unborn branch and a detached HEAD.
pub trait BranchLookups {
    fn name(&self) -> &'static str;
    /// Is there a git repository here at all?
    fn is_repo(&self) -> bool;
    /// Is this repo-relative path (a file or a folder) in HEAD's tree? `false` when HEAD has no commit yet.
    fn path_in_head(&self, path: &str) -> anyhow::Result<bool>;
    /// The branch HEAD is on; `None` when HEAD is detached.
    fn current_branch(&self) -> anyhow::Result<Option<String>>;
    /// Names of the local branches (`main`, `feature/x`), sorted.
    fn local_branches(&self) -> anyhow::Result<Vec<String>>;
}
