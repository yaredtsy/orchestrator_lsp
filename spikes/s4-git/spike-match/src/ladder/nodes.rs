//! Files and folders: only steps 1-3 apply (no body to compare).

use crate::git::Rename;
use crate::model::{FileSnap, Node, Snapshot};

use super::outcome::Outcome;

pub fn locate_file(path: &str, before: &Snapshot, after: &Snapshot, renames: &[Rename]) -> Outcome {
    if after.files.contains_key(path) {
        return Outcome::Found { step: 1, node: Node::File { path: path.into() } };
    }
    let Some(old) = before.files.get(path) else { return Outcome::Missing };
    if let Some((p, _)) = after.files.iter().find(|(p, f)| f.content_hash == old.content_hash && !before.files.contains_key(*p)) {
        return Outcome::Found { step: 2, node: Node::File { path: p.clone() } };
    }
    if let Some(r) = renames.iter().find(|r| r.old == path) {
        return Outcome::Found { step: 3, node: Node::File { path: r.new.clone() } };
    }
    Outcome::Missing
}

pub fn locate_folder(prefix: &str, before: &Snapshot, after: &Snapshot, renames: &[Rename]) -> Outcome {
    let dir = format!("{}/", prefix.trim_end_matches('/'));
    let kids: Vec<(&String, &FileSnap)> = before.files.iter().filter(|(p, _)| p.starts_with(&dir)).collect();
    if kids.is_empty() {
        return Outcome::Missing;
    }
    if after.files.keys().any(|p| p.starts_with(&dir)) {
        return Outcome::Found { step: 1, node: Node::Folder { path: prefix.into() } };
    }
    // step 2: a folder matches by its children's hashes. Try the prefix implied by the first child.
    let (p0, f0) = kids[0];
    let rel0 = &p0[dir.len()..];
    for (q, fq) in &after.files {
        if fq.content_hash == f0.content_hash && q.ends_with(rel0) && q.len() > rel0.len() {
            let new_dir = &q[..q.len() - rel0.len()];
            let all = kids.iter().all(|(p, f)| after.files.get(&format!("{new_dir}{}", &p[dir.len()..])).is_some_and(|a| a.content_hash == f.content_hash));
            if all {
                return Outcome::Found { step: 2, node: Node::Folder { path: new_dir.trim_end_matches('/').into() } };
            }
        }
    }
    // step 3: git saw the children move to one common new folder
    let moved: Vec<&Rename> = renames.iter().filter(|r| r.old.starts_with(&dir)).collect();
    if let Some(first) = moved.first() {
        let rel = &first.old[dir.len()..];
        if first.new.ends_with(rel) && moved.len() * 2 >= kids.len() {
            let new_dir = &first.new[..first.new.len() - rel.len()];
            return Outcome::Found { step: 3, node: Node::Folder { path: new_dir.trim_end_matches('/').into() } };
        }
    }
    Outcome::Missing
}
