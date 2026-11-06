//! VS-D74: Missing vs "not on this branch" is decided by where the node's `node.toml` is, not by how HEAD moved.
//! A pure function over the three git lookups (`BranchLookups`), so it works the same with the git binary, with gix, and in tests.

use crate::git::BranchLookups;

/// What the sync knows about a documented node that is still absent after the ladder.
pub struct NodeFacts<'a> {
    pub archived: bool,
    /// repo-relative path of the node's `node.toml`
    pub node_toml: &'a str,
    /// the last branch the node was seen on (SQLite `last_seen`); `None` = no record
    pub last_seen: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Archived,
    Missing,
    NotOnThisBranch,
}

pub fn node_state(f: &NodeFacts, git: &dyn BranchLookups) -> anyhow::Result<State> {
    if f.archived {
        return Ok(State::Archived);
    }
    if !git.is_repo() {
        return Ok(State::Missing);
    }
    // Committed in HEAD's tree: the docs are orphaned in this commit. No history is needed, so shallow clones work.
    if git.path_in_head(f.node_toml)? {
        return Ok(State::Missing);
    }
    // Otherwise the node FLOATS (private, uncommitted shared, or notes whose shared node.toml left with the branch).
    let Some(seen) = f.last_seen else { return Ok(State::NotOnThisBranch) };
    if git.current_branch()?.as_deref() == Some(seen) {
        return Ok(State::Missing); // HEAD is on the branch where the node was last seen, and it is gone there
    }
    // A detached HEAD counts as a different branch. A renamed branch looks deleted (accepted).
    Ok(if git.local_branches()?.iter().any(|b| b == seen) { State::NotOnThisBranch } else { State::Missing })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        repo: bool,
        in_head: bool,
        branch: Option<&'static str>,
        branches: Vec<&'static str>,
    }
    impl BranchLookups for Fake {
        fn name(&self) -> &'static str {
            "fake"
        }
        fn is_repo(&self) -> bool {
            self.repo
        }
        fn path_in_head(&self, _: &str) -> anyhow::Result<bool> {
            Ok(self.in_head)
        }
        fn current_branch(&self) -> anyhow::Result<Option<String>> {
            Ok(self.branch.map(String::from))
        }
        fn local_branches(&self) -> anyhow::Result<Vec<String>> {
            Ok(self.branches.iter().map(|b| b.to_string()).collect())
        }
    }

    fn facts(archived: bool, last_seen: Option<&'static str>) -> NodeFacts<'static> {
        NodeFacts { archived, node_toml: ".vnoc/nodes/ab/ID/node.toml", last_seen }
    }
    fn git(repo: bool, in_head: bool, branch: Option<&'static str>, branches: &[&'static str]) -> Fake {
        Fake { repo, in_head, branch, branches: branches.to_vec() }
    }

    #[test]
    fn the_rule_table() {
        use State::*;
        let cases: Vec<(&str, NodeFacts, Fake, State)> = vec![
            ("archived wins over everything", facts(true, None), git(true, true, Some("main"), &["main"]), Archived),
            ("no git repository", facts(false, None), git(false, false, None, &[]), Missing),
            ("node.toml committed in HEAD's tree", facts(false, Some("feature")), git(true, true, Some("main"), &["main", "feature"]), Missing),
            ("floating, no last_seen record", facts(false, None), git(true, false, Some("main"), &["main"]), NotOnThisBranch),
            ("floating, HEAD is on the last-seen branch", facts(false, Some("main")), git(true, false, Some("main"), &["main", "feature"]), Missing),
            ("floating, last-seen branch exists elsewhere (a checkout)", facts(false, Some("feature")), git(true, false, Some("main"), &["main", "feature"]), NotOnThisBranch),
            ("floating, last-seen branch was deleted", facts(false, Some("gone")), git(true, false, Some("main"), &["main"]), Missing),
            ("detached HEAD counts as a different branch (it exists)", facts(false, Some("feature")), git(true, false, None, &["main", "feature"]), NotOnThisBranch),
            ("detached HEAD, last-seen branch deleted", facts(false, Some("gone")), git(true, false, None, &["main"]), Missing),
            ("detached HEAD, node.toml in HEAD's tree", facts(false, Some("feature")), git(true, true, None, &["feature"]), Missing),
        ];
        for (name, f, g, want) in cases {
            assert_eq!(node_state(&f, &g).unwrap(), want, "{name}");
        }
    }
}
