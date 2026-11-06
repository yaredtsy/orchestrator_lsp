//! The data the ladder works on.

use std::collections::BTreeMap;

use serde::Deserialize;

/// One def (function or class) as the driver's `vnoc/defs` reports it, plus the file it lives in.
#[derive(Debug, Clone, Deserialize)]
pub struct Def {
    #[serde(skip)]
    pub file: String,
    pub qualname: String,
    pub name: String,
    pub kind: String,
    pub parent: String,
    pub parent_kind: Option<String>,
    pub line: usize,
    pub end_line: usize,
    pub params: Vec<String>,
    pub body_hash: String,
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileSnap {
    pub content_hash: String,
    pub defs: Vec<Def>,
}

/// All .py files of one commit (the "before" or "after" state). The "before" state is what the cache holds in the
/// product; on a fresh clone it is rebuilt from git (C19).
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub files: BTreeMap<String, FileSnap>,
}

impl Snapshot {
    pub fn all_defs(&self) -> impl Iterator<Item = &Def> {
        self.files.values().flat_map(|f| f.defs.iter())
    }
}

/// A documented node the ladder has to find again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Function { path: String, qualname: String },
    File { path: String },
    Folder { path: String },
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Function { path, qualname } => write!(f, "{path}::{qualname}"),
            Node::File { path } => write!(f, "{path}"),
            Node::Folder { path } => write!(f, "{path}/"),
        }
    }
}
