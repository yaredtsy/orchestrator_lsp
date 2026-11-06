//! Build the "before" / "after" state of one commit. In the product the "before" comes from the SQLite cache;
//! on a fresh clone it is rebuilt from git exactly like this (C19).

use crate::driver::Driver;
use crate::git::GitBackend;
use crate::model::{FileSnap, Snapshot};

pub fn snapshot(git: &dyn GitBackend, driver: &mut Driver, commit: &str) -> anyhow::Result<Snapshot> {
    let mut s = Snapshot::default();
    for path in git.list_py_files(commit)? {
        let Some(bytes) = git.read_file(commit, &path)? else { continue };
        let text = String::from_utf8_lossy(&bytes);
        let defs = driver.defs(&path, &text)?;
        s.files.insert(path, FileSnap { content_hash: blake3::hash(&bytes).to_hex().to_string(), defs });
    }
    Ok(s)
}
