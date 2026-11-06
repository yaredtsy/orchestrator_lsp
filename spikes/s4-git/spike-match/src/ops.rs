//! C9: a rebase, merge or cherry-pick in progress. While one runs, reconciling waits and runs once at the end.
//! That is for SPEED only (a rebase rewrites many files in a row); correctness does not depend on it (VS-D74).

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Rebase,
    Merge,
    CherryPick,
}

/// Looks at the git directory the way git itself marks these states. No git process, no gix: four `exists` calls.
pub fn in_progress(git_dir: &Path) -> Option<Operation> {
    let has = |p: &str| git_dir.join(p).exists();
    if has("rebase-merge") || has("rebase-apply") {
        Some(Operation::Rebase)
    } else if has("MERGE_HEAD") {
        Some(Operation::Merge)
    } else if has("CHERRY_PICK_HEAD") {
        Some(Operation::CherryPick)
    } else {
        None
    }
}

/// Feed it every nudge (a HEAD change, a file event) with the current operation state; it says whether to reconcile NOW.
///   no operation running            -> reconcile
///   an operation is running         -> wait, remember that something happened
///   an operation just finished      -> reconcile once, if anything happened meanwhile
#[derive(Default)]
pub struct Pause {
    running: bool,
    dirty: bool,
}

impl Pause {
    pub fn on_event(&mut self, op: Option<Operation>) -> bool {
        match (self.running, op) {
            (_, Some(_)) => {
                self.running = true;
                self.dirty = true;
                false
            }
            (true, None) => {
                self.running = false;
                std::mem::take(&mut self.dirty)
            }
            (false, None) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_during_the_operation_and_runs_once_at_the_end() {
        let mut p = Pause::default();
        assert!(p.on_event(None), "normal: reconcile");
        assert!(!p.on_event(Some(Operation::Rebase)), "rebase starts: wait");
        assert!(!p.on_event(Some(Operation::Rebase)), "many events during it: still waiting");
        assert!(!p.on_event(Some(Operation::Rebase)));
        assert!(p.on_event(None), "it finished: run once");
        assert!(p.on_event(None), "back to normal");
    }

    #[test]
    fn an_operation_that_changed_nothing_does_not_trigger_a_run_at_the_end_twice() {
        let mut p = Pause::default();
        assert!(!p.on_event(Some(Operation::Merge)));
        assert!(p.on_event(None));
        assert!(p.on_event(None)); // plain events are normal again
    }
}
