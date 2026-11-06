//! Real git repositories, both backends. The git binary and gix must agree on the three VS-D74 lookups in every
//! situation (normal, detached, unborn, no repository, shallow clone, in the middle of a rebase), and the VS-D74 rule
//! must give the documented answer. Also: the in-progress operations (C9) are detected from the git directory.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use spike_match::git::{BranchLookups, CliGit, GixGit};
use spike_match::ops::{Operation, in_progress};
use spike_match::reconcile::{NodeFacts, State, node_state};

static N: AtomicU32 = AtomicU32::new(0);

fn tmp(label: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vnoc-s4-{label}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap()
        .status
        .success()
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn commit(dir: &Path, rel: &str, text: &str, msg: &str) {
    write(dir, rel, text);
    assert!(git(dir, &["add", "-A"]));
    assert!(git(dir, &["commit", "-q", "-m", msg]));
}

/// A repo on `main` with a committed shared node and a `feature` branch that has one more commit.
fn repo() -> PathBuf {
    let d = tmp("repo");
    assert!(git(&d, &["init", "-q", "-b", "main"]));
    commit(&d, "src/a.py", "def f():\n    return 1\n", "a");
    commit(&d, ".vnoc/nodes/ab/ID1/node.toml", "path = \"src/a.py\"\n", "node");
    assert!(git(&d, &["branch", "feature"]));
    d
}

/// Both backends, asserted equal; returns the answer.
fn both<T: PartialEq + std::fmt::Debug>(dir: &Path, what: &str, f: impl Fn(&dyn BranchLookups) -> T) -> T {
    let (c, g) = (CliGit::new(dir), GixGit::new(dir));
    let (a, b) = (f(&c), f(&g));
    assert_eq!(a, b, "{what}: the git binary and gix disagree");
    a
}

#[test]
fn lookups_agree_in_a_normal_repo() {
    let d = repo();
    assert!(both(&d, "is_repo", |g| g.is_repo()));
    assert!(both(&d, "file in HEAD", |g| g.path_in_head("src/a.py").unwrap()));
    assert!(both(&d, "folder in HEAD", |g| g.path_in_head(".vnoc/nodes").unwrap()));
    assert!(both(&d, "nested file in HEAD", |g| g.path_in_head(".vnoc/nodes/ab/ID1/node.toml").unwrap()));
    assert!(!both(&d, "absent path", |g| g.path_in_head("src/nope.py").unwrap()));
    assert_eq!(both(&d, "branch", |g| g.current_branch().unwrap()), Some("main".to_string()));
    assert_eq!(both(&d, "branches", |g| g.local_branches().unwrap()), vec!["feature", "main"]);
}

#[test]
fn a_branch_name_with_a_slash() {
    let d = repo();
    assert!(git(&d, &["checkout", "-q", "-b", "feature/x"]));
    assert_eq!(both(&d, "branch", |g| g.current_branch().unwrap()), Some("feature/x".to_string()));
    assert_eq!(both(&d, "branches", |g| g.local_branches().unwrap()), vec!["feature", "feature/x", "main"]);
}

#[test]
fn a_detached_head_has_no_branch() {
    let d = repo();
    assert!(git(&d, &["checkout", "-q", "--detach", "HEAD"]));
    assert_eq!(both(&d, "branch", |g| g.current_branch().unwrap()), None);
    assert!(both(&d, "file still in HEAD", |g| g.path_in_head("src/a.py").unwrap()));
}

#[test]
fn an_unborn_branch_has_a_name_and_no_tree() {
    let d = tmp("unborn");
    assert!(git(&d, &["init", "-q", "-b", "main"]));
    assert!(both(&d, "is_repo", |g| g.is_repo()));
    assert!(!both(&d, "path", |g| g.path_in_head("a.py").unwrap()));
    assert_eq!(both(&d, "branch", |g| g.current_branch().unwrap()), Some("main".to_string()));
    assert!(both(&d, "branches", |g| g.local_branches().unwrap()).is_empty());
}

#[test]
fn no_repository() {
    let d = tmp("norepo");
    assert!(!both(&d, "is_repo", |g| g.is_repo()));
}

#[test]
fn a_shallow_clone_needs_no_special_case() {
    let src = repo();
    commit(&src, "src/b.py", "x = 1\n", "b");
    let dst = tmp("shallow");
    let out = Command::new("git").args(["clone", "-q", "--depth", "1", &format!("file://{}", src.display())]).arg(&dst).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(dst.join(".git/shallow").exists(), "this really is a shallow clone");
    assert!(both(&dst, "file in HEAD", |g| g.path_in_head("src/b.py").unwrap()));
    assert_eq!(both(&dst, "branch", |g| g.current_branch().unwrap()), Some("main".to_string()));
    assert_eq!(both(&dst, "branches", |g| g.local_branches().unwrap()), vec!["main"]);
}

/// The documented answer of VS-D74 in the situations of the plan (journey E3, E4, E5), with real git and both backends.
#[test]
fn the_rule_in_real_situations() {
    let shared = ".vnoc/nodes/ab/ID1/node.toml";
    let floating = ".vnoc/local/nodes/cd/ID2/node.toml"; // never committed
    for backend in ["cli", "gix"] {
        let d = repo();
        let state = |f: &NodeFacts| -> State {
            if backend == "cli" { node_state(f, &CliGit::new(&d)).unwrap() } else { node_state(f, &GixGit::new(&d)).unwrap() }
        };
        let (shared_node, floating_seen_on_feature) = (NodeFacts { archived: false, node_toml: shared, last_seen: Some("main") }, NodeFacts { archived: false, node_toml: floating, last_seen: Some("feature") });

        // E3/E5: a pulled deletion of the CODE: the shared node.toml is still committed in HEAD
        assert_eq!(state(&shared_node), State::Missing, "{backend}: committed node.toml, code gone");
        // E4: checkout to another branch; a floating node last seen on `feature` is just elsewhere
        assert!(git(&d, &["checkout", "-q", "main"]));
        assert_eq!(state(&floating_seen_on_feature), State::NotOnThisBranch, "{backend}: floating, last seen on feature, HEAD on main");
        // back on its own branch, it is gone there
        assert!(git(&d, &["checkout", "-q", "feature"]));
        assert_eq!(state(&floating_seen_on_feature), State::Missing, "{backend}: HEAD on the last-seen branch");
        // the last-seen branch is deleted
        assert!(git(&d, &["checkout", "-q", "main"]));
        assert!(git(&d, &["branch", "-q", "-D", "feature"]));
        assert_eq!(state(&floating_seen_on_feature), State::Missing, "{backend}: last-seen branch deleted");
        // a shared node.toml that is NOT in HEAD's tree behaves like a floating one: after `git rm` of it
        assert!(git(&d, &["rm", "-q", shared]));
        assert!(git(&d, &["commit", "-q", "-m", "teammate removed the node file"]));
        assert_eq!(state(&NodeFacts { archived: false, node_toml: shared, last_seen: None }), State::NotOnThisBranch, "{backend}: not in HEAD, no record");
    }
}

// ── C9: operations in progress ──────────────────────────────────────────────────────────────────

/// Two branches that change the same line differently.
fn conflicting() -> PathBuf {
    let d = tmp("conflict");
    assert!(git(&d, &["init", "-q", "-b", "main"]));
    commit(&d, "f.txt", "one\n", "base");
    assert!(git(&d, &["checkout", "-q", "-b", "b"]));
    commit(&d, "f.txt", "from b\n", "b");
    assert!(git(&d, &["checkout", "-q", "main"]));
    commit(&d, "f.txt", "from main\n", "main");
    d
}

#[test]
fn a_rebase_in_progress_is_seen_and_ends() {
    let d = conflicting();
    let gd = CliGit::new(&d).git_dir().unwrap();
    assert_eq!(in_progress(&gd), None);
    assert!(git(&d, &["checkout", "-q", "b"]));
    assert!(!git(&d, &["rebase", "main"]), "the rebase stops on a conflict");
    assert_eq!(in_progress(&gd), Some(Operation::Rebase));
    // during a rebase HEAD is detached: no current branch, with both backends (so VS-D74 treats it as a different branch)
    assert_eq!(both(&d, "branch during a rebase", |g| g.current_branch().unwrap()), None);
    assert!(git(&d, &["rebase", "--abort"]));
    assert_eq!(in_progress(&gd), None);
    assert_eq!(both(&d, "branch after", |g| g.current_branch().unwrap()), Some("b".to_string()));
}

#[test]
fn a_merge_in_progress_is_seen_and_ends() {
    let d = conflicting();
    let gd = CliGit::new(&d).git_dir().unwrap();
    assert!(!git(&d, &["merge", "b"]));
    assert_eq!(in_progress(&gd), Some(Operation::Merge));
    // a merge does NOT detach HEAD
    assert_eq!(both(&d, "branch during a merge", |g| g.current_branch().unwrap()), Some("main".to_string()));
    assert!(git(&d, &["merge", "--abort"]));
    assert_eq!(in_progress(&gd), None);
}

#[test]
fn a_cherry_pick_in_progress_is_seen_and_ends() {
    let d = conflicting();
    let gd = CliGit::new(&d).git_dir().unwrap();
    assert!(!git(&d, &["cherry-pick", "b"]));
    assert_eq!(in_progress(&gd), Some(Operation::CherryPick));
    assert_eq!(both(&d, "branch during a cherry-pick", |g| g.current_branch().unwrap()), Some("main".to_string()));
    assert!(git(&d, &["cherry-pick", "--abort"]));
    assert_eq!(in_progress(&gd), None);
}
