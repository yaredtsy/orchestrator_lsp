//! The labelled set and the corpus it was drawn from.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use spike_match::driver::Driver;
use spike_match::git::{CliGit, GitBackend};
use spike_match::model::Def;

/// One mutated version of one real function (written by `gen_labelled.py`).
#[derive(Deserialize)]
pub struct Case {
    pub file: String,
    pub qualname: String,
    pub parent: String,
    pub parent_kind: Option<String>,
    pub new_name: String,
    /// how much of the body was edited (0.0 = renamed only)
    pub level: f32,
    pub new_source: String,
}

#[derive(Deserialize)]
struct Dataset {
    cases: Vec<Case>,
}

pub struct Repo {
    pub name: &'static str,
    pub path: PathBuf,
}

pub fn load_cases(data: &Path, name: &str) -> anyhow::Result<Vec<Case>> {
    let ds: Dataset = serde_json::from_slice(&std::fs::read(data.join(format!("{name}.json")))?)?;
    Ok(ds.cases)
}

/// Every def of the repo's committed `HEAD` (never the working tree).
pub fn load_corpus(git: &CliGit, driver: &mut Driver) -> anyhow::Result<Vec<Def>> {
    let mut all = vec![];
    for f in git.list_py_files("HEAD")? {
        if let Some(b) = git.read_file("HEAD", &f)? {
            all.extend(driver.defs(&f, &String::from_utf8_lossy(&b))?);
        }
    }
    Ok(all)
}
