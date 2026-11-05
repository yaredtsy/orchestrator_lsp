# 05.04: S4 git and matching

**Goal:** docs survive refactors and branch switches in `none` mode (no in-code IDs): the MVP's hardest promise.
**Answers:** B6 (VS Code nudges, Rust reads git), C9 (`vscode.git` API behaviour), C14 (gix speed: ancestry, branch tips, shallow clones), C19 (fresh-clone "before" state),
VS-O10 (the similarity threshold).
**Time box:** 3 days. **Code:** `spikes/s4-git/`. **Needs:** S1 (S2 helps for real outlines; a tiny fake outline is fine).

## Build

1. **HEAD awareness.** Rust reads HEAD with `gix`, stores `last_synced_head`, runs the ancestry test (02-05 §5). The extension subscribes to
   `vscode.git`'s `repository.state.onDidChange` and only sends a nudge.
2. **The branch cases**, each with VS Code **open** and **closed**:
   - checkout to a branch without the function → *not on this branch*, no prompt;
   - pull a teammate's commit that deleted it → *Missing* + prompt;
   - rebase onto an upstream that deleted it → *not on this branch*, then **promoted to Missing** when the list is opened (tip check).
3. **C9:** what does the git API report during a rebase or merge in progress, and with `git.enabled = false`? Does our nudge still arrive?
4. **C14:** time `is_ancestor` on a big history (e.g. cpython). Time reading one path from every local branch tip (the tip check). Try a `--depth 1` clone.
5. **The refactor fixtures.** A small Python repo with a doc on several nodes, then one commit per refactor:
   rename a function · move a file · rename a folder · move **and** edit a file · move a function to another file · rename a function **and** change its body.
   Run the ladder (02-05 §4) after each and record **which step** matched, or whether it fell to Missing.
6. **VS-O10:** for the similarity step, record the scores of right and wrong candidates on the fixtures. Pick the threshold that gives **zero wrong silent matches**.
7. **C19, fresh clone:** delete the cache and rebuild from git: find "the last commit that changed this `node.toml`" with gix and read the file at that commit. Then the same on a shallow clone.

> **🦀 Rust note: the ancestry test with gix.**
> `git merge-base --is-ancestor old new` asks: "is `old` reachable by walking parents from `new`?" With gix that's a walk over the commit graph from `new`, stopping when it
> finds `old` or runs out of commits older than `old`. gix can use the **commit-graph** file (`.git/objects/info/commit-graph`) when present, which is what keeps this fast on
> big histories. Step 4 measures whether that's enough.

## Measure

- `is_ancestor` and tip-check times (C14); the fresh-clone rebuild time for 100 documented nodes (C19).
- A table: refactor × ladder step that matched (or Missing). This table becomes the MVP's refactor test suite (04-01 exit criterion 2).

## Pass / fail

- ✅ No branch switch prompts; the pulled deletion prompts; the rebase case is promoted.
- ✅ Zero wrong silent matches on the fixtures; a threshold chosen for VS-O10.
- ✅ B6, C9, C14, C19 have answers below.

## Findings

*(empty until S4 runs)*

Next: [`05-canvas-editors.md`](05-canvas-editors.md)
