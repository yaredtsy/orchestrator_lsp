//! Who is still unmatched once steps 1-3 have run. Step 4 needs this: a body hash may only match silently when it is
//! unique among the unmatched OLD functions AND among the unmatched NEW ones. Two identical methods that were both
//! renamed in one commit are therefore never matched by hash alone.

use std::collections::{HashMap, HashSet};

use crate::git::Rename;
use crate::model::{Def, Snapshot};

use super::config::Steps;

pub fn find<'a>(s: &'a Snapshot, path: &str, qualname: &str) -> Option<&'a Def> {
    s.files.get(path)?.defs.iter().find(|d| d.qualname == qualname)
}

pub struct Matching {
    old_hash: HashMap<String, usize>,
    new_hash: HashMap<String, usize>,
    /// (file, qualname) of new defs already claimed by steps 1-3
    claimed: HashSet<(String, String)>,
}

impl Matching {
    /// True when exactly one unmatched old and exactly one unmatched new function have this body.
    pub fn is_unique(&self, body_hash: &str) -> bool {
        self.old_hash.get(body_hash) == Some(&1) && self.new_hash.get(body_hash) == Some(&1)
    }
    pub fn is_claimed(&self, file: &str, qualname: &str) -> bool {
        self.claimed.contains(&(file.to_string(), qualname.to_string()))
    }
}

pub fn matching(before: &Snapshot, after: &Snapshot, renames: &[Rename], steps: &Steps) -> Matching {
    let mut claimed: HashSet<(String, String)> = HashSet::new();
    let mut old_hash: HashMap<String, usize> = HashMap::new();
    for (path, file) in &before.files {
        // where did this file go? (step 2: same content hash at a new path; step 3: git rename detection)
        let mut homes: Vec<&str> = vec![path.as_str()];
        if steps.file_moves {
            homes.extend(after.files.iter().filter(|(p, f)| *p != path && f.content_hash == file.content_hash && !before.files.contains_key(*p)).map(|(p, _)| p.as_str()));
            homes.extend(renames.iter().filter(|r| r.old == *path).map(|r| r.new.as_str()));
        }
        for d in &file.defs {
            match homes.iter().find(|h| find(after, h, &d.qualname).is_some()) {
                Some(h) => {
                    claimed.insert((h.to_string(), d.qualname.clone()));
                }
                None => *old_hash.entry(d.body_hash.clone()).or_default() += 1,
            }
        }
    }
    let existed: HashSet<(&str, &str)> = before.all_defs().map(|d| (d.file.as_str(), d.qualname.as_str())).collect();
    let mut new_hash: HashMap<String, usize> = HashMap::new();
    for d in after.all_defs() {
        if !existed.contains(&(d.file.as_str(), d.qualname.as_str())) && !claimed.contains(&(d.file.clone(), d.qualname.clone())) {
            *new_hash.entry(d.body_hash.clone()).or_default() += 1;
        }
    }
    Matching { old_hash, new_hash, claimed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{def, snap};

    const BODY: &str = "total = 0 . 0 for item in items : total += item [ 'p' ] return round ( total , 2 )";

    #[test]
    fn two_identical_methods_renamed_together_are_not_unique() {
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY), def("a.py", "C.net", BODY)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY), def("a.py", "C.net_total", BODY)])]);
        let m = matching(&before, &after, &[], &Steps::all());
        assert!(!m.is_unique(&before.files["a.py"].defs[0].body_hash));
    }

    #[test]
    fn one_renamed_method_is_unique() {
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY)])]);
        let m = matching(&before, &after, &[], &Steps::all());
        assert!(m.is_unique(&before.files["a.py"].defs[0].body_hash));
    }

    #[test]
    fn a_twin_that_still_matches_is_not_counted_as_unmatched() {
        // gross is renamed, net is untouched: only gross is unmatched on the old side
        let before = snap(&[("a.py", "h1", vec![def("a.py", "C.gross", BODY), def("a.py", "C.net", BODY)])]);
        let after = snap(&[("a.py", "h2", vec![def("a.py", "C.gross_total", BODY), def("a.py", "C.net", BODY)])]);
        let m = matching(&before, &after, &[], &Steps::all());
        assert!(m.is_unique(&before.files["a.py"].defs[0].body_hash));
        assert!(m.is_claimed("a.py", "C.net"));
    }
}
