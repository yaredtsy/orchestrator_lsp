//! Small builders for unit tests (compiled only for `cargo test`).

use crate::model::{Def, FileSnap, Snapshot};

/// A def whose tokens are the whitespace-separated words of `body`.
pub fn def(file: &str, qualname: &str, body: &str) -> Def {
    let tokens: Vec<String> = body.split_whitespace().map(String::from).collect();
    let (parent, name) = qualname.rsplit_once('.').map_or(("", qualname), |(p, n)| (p, n));
    Def {
        file: file.into(),
        qualname: qualname.into(),
        name: name.into(),
        kind: "function".into(),
        parent: parent.into(),
        parent_kind: (!parent.is_empty()).then(|| "class".to_string()),
        line: 1,
        end_line: 1,
        params: vec![],
        body_hash: blake3::hash(tokens.join("\u{1f}").as_bytes()).to_hex().to_string(),
        tokens,
    }
}

pub fn snap(files: &[(&str, &str, Vec<Def>)]) -> Snapshot {
    let mut s = Snapshot::default();
    for (path, hash, defs) in files {
        s.files.insert(path.to_string(), FileSnap { content_hash: hash.to_string(), defs: defs.clone() });
    }
    s
}
