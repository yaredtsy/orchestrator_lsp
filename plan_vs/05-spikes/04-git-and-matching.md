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

*In progress (2026-11-06). Done: the ladder, the six fixtures (+ 2 extra), VS-O10 on the labelled set. Not done yet: gix and the django timings (C14), C9 and the pause rule.
Code: `spikes/s4-git/` (`spike-match`: `fixtures`, `tune`; `gen_labelled.py`). The S2 driver gained `vnoc/defs` (per-def fingerprints).*

**Ladder fixtures (git CLI behind `GitBackend`; nine documented nodes; both scores behave the same here):**
| Refactor | Result |
|---|---|
| rename a function | **step 4** (same body) |
| move a file | **step 2** (same file hash) |
| rename a folder | **step 2**, for functions, the file and the folder node (children's hashes) |
| move AND edit a file | **step 3** (git rename detection, `-M`) |
| move a function to another file | **step 4** |
| rename a function AND rewrite its body | **asked** when the ask-floor is 0 (the right function is the top candidate, jaccard 0.03, edit 0.32); **Missing** at the default floor 0.5 |
| delete a function whose sibling looks the same (extra) | **Missing**, not matched to the sibling (the sibling existed before, so it is no candidate) |
| two identical methods, both renamed in one commit (extra, VS-D73) | **both ask**, the likelier candidate first, **nothing silent** |
**No wrong silent match** in any fixture.

**VS-O10 on the labelled set** (real functions from LightRAG `HEAD`, httpx, requests, flask; 120 per repo, each renamed and edited 0-70%, placed in the same scope / module level / another file, plus a deleted variant;
~9,600 cases; tuned on LightRAG + httpx, validated on requests + flask; the silent rule needs score, margin, same file or class, and a minimum token count):
- Zero wrong silent matches is reachable, but only at a very high bar: **Edit score >= 0.99, Jaccard >= 0.91** (24 and 212 of 1,440 grid points). **Held-out: 0 wrong silent matches for both.**
- **There is a cliff, not a slope:** lowering the Edit threshold by 0.02 gives 186 wrong silent matches (train + held-out), by 0.05 gives 399.
- **So in practice only an edit-free rename or move is matched silently** (Edit, same scope: 317 of 480 at 0% edit, 20 of 476 at 5%, ~0 above 10%). **Anything edited is asked.**
- The right answer is usually offered first when asked: the true successor's mean Edit score is 0.91 at 5% edits and 0.62 at 35%, against 0.46 for the best wrong candidate.
- **Edit is the better score for asking**: with Jaccard, about 1,300 of 4,900 training cases fall below the ask floor and show as Missing; with Edit, none do.
- Names and parameters: name similarity of the true successor is flat at 0.64 (the generator renames every function), the parameter overlap falls with the edit level (1.00 to 0.04).
- **Consequence for the default ID mode:** without in-code IDs, a function that is renamed **and** edited in one commit becomes a "Find it…" question. Nothing is lost, and the answer is nearly always first in the list.
- **Twin cases** (the lost function has an identical twin in the corpus; 191 of 4,884 train cases, 388 of 4,711 held-out): under VS-D73 alone, **95 of 191 train twin cases and 51 of 388 held-out ones are silently wrong** (a new copy of the still-existing twin looks like the lost function's successor).
  Experiment: if a lost body with an identical twin anywhere **only asks** (never silent), twin wrong goes to **0** and silent-right drops from 446 to 444 (Edit) / 530 to 526 (Jaccard). Not adopted yet: waiting for your decision.



Next: [`05-canvas-editors.md`](05-canvas-editors.md)
