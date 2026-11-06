# 05.04: S4 git and matching

**Goal:** docs survive refactors and branch switches in `none` mode (no in-code IDs): the MVP's hardest promise.
**Answers:** B6 (VS Code nudges, Rust reads git), C9 (`vscode.git` API behaviour + the pause rule), C14 (gix: ancestry speed, shallow behaviour), VS-O10 (the similarity rule).
**Dropped (VS-D72):** C19 (the fresh-clone history walk) and the per-branch-tip file check.
**Time box:** 3 days. **Code:** `spikes/s4-git/`. **Needs:** S1; the S2 driver for fingerprints (`vnoc/defs`).

**Order (decided):** (1) the ladder first, because its results may change the default ID mode; (2) gix; (3) C9. Git sits behind a small `GitBackend` trait; the first implementation calls the `git` binary.

## Build

1. **The ladder + six refactor fixtures, plain `git` behind `GitBackend`.** A small Python repo with documented nodes, one branch per refactor:
   rename a function · move a file · rename a folder · move **and** edit a file · move a function to another file · rename a function **and** change its body
   (+ an extra: delete a function whose sibling looks the same). Run the ladder (02-05 §4) after each and record **which step** matched, or whether it was asked or Missing, and any wrong or missed match.
2. **VS-O10, the similarity rule.** Measure **two scores** per candidate: token 3-gram Jaccard and normalized token edit distance on the stripped body. Next to them record the **context features**:
   same file, same class, parameter-name overlap, name similarity. **A silent match needs all of:** score above a threshold, a **margin** over the second best, **same file or same class**, and a body of at least a
   **minimum token count**. Anything else asks the user. **Rule: zero wrong silent matches** (a wrong silent match is worse than a question). Tune on a **labelled set** built from real functions of open-source repos
   (`gen_labelled.py`: each function renamed and edited by 0-70%, placed in the same scope / another file / module level, plus a "deleted" variant), tune on some repos and **validate on others**.
3. **gix behind the same trait:** ancestry, branch tips, `exists`/read a file at a commit. Anything slow or missing **falls back to the git binary through the same trait**; record which operations needed the fallback.
4. **C14, timings:** `is_ancestor` on django **with and without a commit-graph**. A shallow copy (LightRAG) must show that ancestry **fails safely** when history is missing (no panic, no wrong state).
5. **The HEAD-change rule (VS-D72):** new HEAD descends from old → *Missing*; else old HEAD still the tip of a local branch → *not on this branch*; else → *Missing*. No history walk, no file reads. Test each branch case with the project **open and closed**:
   checkout to a branch without the function · pull a teammate's deletion · rebase onto an upstream that deleted it · reset.
6. **C9 and the pause rule.** What does `vscode.git` report during a rebase or merge in progress, and with `git.enabled = false`? Does our nudge still arrive? **Rule:** while a rebase, merge or cherry-pick is in progress
   (`.git/rebase-merge`, `.git/rebase-apply`, `MERGE_HEAD`, `CHERRY_PICK_HEAD`), **pause reconciling and run it once when the operation finishes.**

> **🦀 Rust note: the ancestry test with gix.**
> `git merge-base --is-ancestor old new` asks: "is `old` reachable by walking parents from `new`?" With gix that's a walk over the commit graph from `new`, stopping when it
> finds `old` or runs out of commits older than `old`. gix can use the **commit-graph** file (`.git/objects/info/commit-graph`) when present, which is what keeps this fast on
> big histories. Step 4 measures whether that's enough.

## Measure

- `is_ancestor` time with and without a commit-graph (C14); which gix operations needed the git-binary fallback.
- A table: refactor × ladder step that matched (or Missing). This table becomes the MVP's refactor test suite (04-01 exit criterion 2).

## Pass / fail

- ✅ No branch switch prompts; the pulled deletion prompts; rebase and reset go straight to Missing (VS-D72).
- ✅ Zero wrong silent matches on the fixtures; a threshold chosen for VS-O10.
- ✅ B6, C9, C14 have answers below; VS-O10 has a rule whose threshold was validated on repos it was not tuned on.
- ✅ The pause rule: a rebase, merge or cherry-pick in progress pauses reconciling, and it runs once at the end.

## Findings

*(empty until S4 runs)*

Next: [`05-canvas-editors.md`](05-canvas-editors.md)
