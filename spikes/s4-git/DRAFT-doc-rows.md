# Draft rows for the docs you maintain (not applied)

Written 2026-11-06 during spike S4. Nothing here has been put into `01-decision-log.md`, `01-review-checklist.md` or `05-spikes/`. Every draft is in this one file: §0 VS-D74, §1 VS-D75, §2 the VS-D73 amendment, §3 checklist, §4 findings, §5 sweep (VS-D75), §6 VS-D76, §7 VS-D77, §8 sweep (VS-D76/D77), §9 findings for the remaining S4 checks.

## 0. Decision log: row VS-D74 (reconstructed: please check it against your original)

I did not have the text of your VS-D74 row. This is rebuilt from how the home docs describe it (`02-technology/05-identity-and-archive.md` §5, `02-technology/04-storage-and-git.md` §5, FR-79). Replace it with your wording if it differs.

| VS-D74 | **[you] Missing vs "not on this branch" is decided by where the node's `node.toml` is, not by how HEAD moved** (replaces the ancestry test of VS-D34 and the ref check of VS-D72). For each documented node still absent after the ladder: **archived** → Archived; **no git repository** → Missing; its **`node.toml` is committed in HEAD's tree** → **Missing** (its docs are orphaned in this commit; no history needed, works on shallow clones); otherwise the node **floats** (private, uncommitted shared, or private notes whose shared `node.toml` left with the branch): **no `last_seen` record** → not on this branch; **HEAD is on the last-seen branch** → Missing; **the last-seen branch still exists** → not on this branch; **the last-seen branch was deleted** → Missing. **`last_seen(id, branch)`** (SQLite) is updated on every sync where the node is present and HEAD is on a branch; a **detached HEAD never updates it** and counts as a different branch. Accepted edge: a renamed branch looks deleted. **What Rust reads from git** (`gix`): a path lookup in HEAD's tree, the current branch, the list of local branches; no ancestry, no history. **Anchor rewrite** (`path` / `qualname` in `node.toml`) after the ladder finds a move: a **committed** `node.toml` only for moves in **uncommitted working-tree changes** (a move already committed in HEAD stays in SQLite and the node offers **"Update anchors"**); a **floating** `node.toml` only when HEAD is on the node's last-seen branch; **never on a detached HEAD**, never for "not on this branch". The grouped Missing notice is **not shown while HEAD is detached**. **Pause** while a rebase, merge or cherry-pick is in progress: reconcile once at the end (speed only, correctness does not depend on it, C9). | supersedes VS-D72 (2) and the ancestry parts of VS-D34; refines VS-D45 | 02-05 §5, 02-04 §5 |

## 1. Decision log: new row VS-D75 (the provisional tier: built, off in v1)

| VS-D75 | **[you] A middle tier between a silent match and a question: the provisional match. Built, off in v1.** The code stays; `Config::provisional` is `None` by default. **v1 behaviour: a strict silent match, or ask with the top candidate preselected.** **No floor is set** (no floor reached 95% on the synthetic labelled set; neither a 90% floor nor a near-twin rule for now). **Revisit with real tester data**, measured on **unobserved moves only** (git renames, agent disk writes, cut/paste), with **caller transfer + same file/class + near-twin as gates**; **the target stays 95%**. **Design, for when it is switched on:** the top candidate passes a provisional floor, has a clear lead over the second, and no ask-only rule applies; shown as an **"Unconfirmed link"** with **Confirm**, **pick another candidate**, **Detach** (back to Missing). **Provisional only when the node would otherwise be Missing** (never for "not on this branch" or Archived). State lives **only in SQLite**; `node.toml` keeps the old anchor until Confirm, which **works like "Update anchors"** (the shared action writes the new anchor). **Teammates may see different provisional matches** (the inputs differ per machine): harmless, because it is SQLite-only and labeled. **No ID is written into code** until Confirm. **MCP:** `node_context` returns `link: "unconfirmed"`. **One grouped notice per sync** with "Confirm all". **Labels never expire or auto-approve.** | refines VS-D44, VS-D73, VS-D74 | 05-spikes/04 |

## 2. Decision log: amendment to VS-D73

Add to the end of the row: "**Ask-only rules** (they cap a match at *ask*: never silent, never provisional; they apply to steps 4 and 5): **(a)** an **exact twin of the lost body anywhere in the before-state or the after-state, matched or unmatched** (in code: another function with the same body before, or the body appearing twice or more after; the candidate of a clean rename is not a twin); **(b)** a body under the **minimum token count**. Measured in S4: with the twin rule on, silently wrong twin cases go from 95 of 191 (train) to 0, at a cost of 2 silent matches (446 to 444)."

## 3. Checklist

No new row needed. VS-O10 can be marked answered by the S4 findings once you accept them (rule: silent only at Edit score >= 0.99; see findings).

## 4. S4 findings: provisional tier (paste into `05-spikes/04-git-and-matching.md`)

*Measured on the synthetic labelled set (functions renamed and edited by a script). Not a decision input for v1: the tier is built and off (VS-D75). Re-measure on unobserved moves from real testers.*

**Code was restructured first, with no behaviour change** (the fixtures and 82 lines of tuning output are identical to the saved baseline), so the tiers are now one pure function (`ladder/policy.rs`, with tests) and the tuning harness calls the same `resolve` the ladder uses. See `spikes/s4-git/spike-match/README.md`.

**Measurement.** Cases that stay a *soft* question under the silent rule (the score or the lead is not good enough; not a twin, not a short body): 4,086 train, 3,729 held-out. For each, the top candidate by Edit score.

Top-1 accuracy per Edit-score band, **lead over the second >= 0.10** (train / held-out):
| band | train | held-out |
|---|---|---|
| 0.50-0.55 | 95% (94 cases) | 99% (154) |
| 0.55-0.60 | 98% (327) | 99% (424) |
| 0.60-0.65 | 100% (425) | 98% (484) |
| 0.65-0.70 | 93% (238) | 92% (202) |
| 0.75-0.80 | 91% (335) | 97% (301) |
| 0.80-0.85 | 71% (160) | 79% (205) |
| 0.85-0.90 | 77% (452) | 94% (371) |
| 0.90-0.95 | 70% (699) | 90% (527) |
| 0.95-1.00 | 68% (379) | 100% (277) |

**No floor reaches 95% top-1 accuracy.** Accuracy is **not monotone in the score**: a floor includes every band above it, and the high bands are the polluted ones. What a floor can reach (train / held-out):
| target | floor / lead | links | accuracy |
|---|---|---|---|
| 99%, 95% | none reaches it | | |
| 90% | 0.50 / 0.30 | 2,183 / 2,397 | 91.0% / 98.4% |
| 85% | 0.45 / 0.20 | 2,681 / 2,757 | 85.6% / 96.8% |
| 80% | 0.30 / 0.10 | 3,231 / 3,115 | 82.3% / 93.9% |

**Why the high bands are wrong** (Edit >= 0.80, lead >= 0.10): edits of 0-5%: **654 of 654 right** (train), 658 of 658 (held-out). Edits of 10-20%: 87% / 97% right. Edits of **35%+: 9 of 367 right** (0 of 86 held-out): the true successor only scores ~0.6, so a candidate at >= 0.80 is a lookalike. The simulation plants one on purpose (a copy of the closest sibling in the same file). In the 41 + 11 *deleted* cases any link is wrong.

**What would move from ask to provisional** (of 4,354 asked, train / of 4,134, held-out):
| floor / lead | moved | right | wrong (of which for a deleted function) |
|---|---|---|---|
| 0.50 / 0.30 | 2,183 / 2,397 | 1,987 / 2,359 | 196 (42) / 38 (14) |
| 0.45 / 0.20 | 2,681 / 2,757 | 2,296 / 2,670 | 385 (54) / 87 (27) |
| 0.30 / 0.10 | 3,231 / 3,115 | 2,659 / 2,924 | 572 (80) / 191 (44) |
Silent matches are unchanged (444 train, 423 held-out).

On the refactor fixtures, with floor 0.30 / lead 0.10 the one miss (rename a function **and** rewrite its body) becomes a **provisional, right** link; the twin methods still ask (`[Twin]`).

**Decided:** no floor in v1 (VS-D75). Neither the 90% floor (0.50 / lead 0.30) nor a near-twin rule is adopted for now; both stay as ideas for the real-data revisit.

## 5. Sweep list for VS-D75 (hand to the other Claude)

The tier is built and off in v1, so the docs describe v1 behaviour and only mention the tier.
- `02-technology/05-identity-and-archive.md` §4 (the ladder): one short paragraph after the ladder table: **provisional tier: built, off in v1 (VS-D75)**. v1: a strict silent match, or ask with the top candidate preselected. Do not describe the label or its actions as v1 behaviour.
- `02-technology/03-frontend-webview.md`: the "Find it…" list **preselects the top candidate**.
- No FR, no user-journey row and no UI label for the tier in v1.
- Summary docs (README, `04-roadmap/01-mvp-vs-stable.md`): one line linking VS-D75. MCP `node_context` returning `link: "unconfirmed"` stays in the decision row only (phase 4).
- Do **not** write an accuracy figure or a floor value anywhere.

## 6. Decision log: new row VS-D76 (observed moves)

| VS-D76 | **[you] Step 0 before the ladder: observed moves.** **Principle: step 0 can only fail toward the ladder, never toward a link.** Any edit that isn't clearly typed or applied in the editor ends tracking for that document. **Range tracking:** the core keeps each node's range in open documents and shifts it on every incremental `didChange`; after reparse, the function at a tracked range is the same node, even if its name and body changed. **Disk reloads** (git checkout, an agent writing an open file) end tracking for that document. Reload test **(spike check)**: not dirty after the change + a watcher event for the same file + buffer text equals disk content. Undo back to the saved state has no watcher event, so it counts as an editor edit. **Whole-node or whole-document edits** (paste-over, select-all + retype, formatters): moves observed before the edit stay valid; after it, tracking resumes only for nodes that step 1 matches by exact `path::qualname` in the new text (formatters never rename). An edit spanning two nodes ends tracking for both. **Unsaved moves** are pending, **memory only**, and become real when that document version is saved; dropped on restart (the ladder handles the file). **File renames:** `onWillRenameFiles` / `onDidRenameFiles` update the anchors old → new directly (a folder rename covers every node under it); `git mv` and a terminal `mv` go to the ladder. **Saved observed moves are stored in SQLite and applied first at sync**: exact, no scores. **The ladder runs only for unobserved changes:** git, disk writes by agents or other tools, edits while VS Code was closed, and cut/paste between files (no rule: step 4 catches identical bodies). **Anchors:** an observed move is an uncommitted working-tree move, so VS-D74's rewrite rule applies (it may rewrite a committed `node.toml`; never on a detached HEAD). **Spike checks:** the reload test; which Python formatter extensions return one whole-document edit vs minimal edits. **Re-measure the ladder on unobserved moves only** (git renames, agent disk writes, cut/paste) before choosing any provisional floor (VS-D75). | refines VS-D44 (adds step 0, makes rename events authoritative), VS-D75 (floor waits for the re-measure) | 02-05 §4, 06-core/05, 08-extension/03 |

## 7. Decision log: new row VS-D77 (doc freshness)

| VS-D77 | **[you] Doc freshness.** When a doc is saved, store the node's **normalised body hash** (whitespace, comments, docstrings and V-NOC `ID:` lines stripped) **in that doc's frontmatter**: per doc, so it travels with the doc to teammates and fresh clones. Not in `node.toml` (it may be on the other side from a private doc, and it's shared by every doc on the node) and not in SQLite (per machine). If the current hash differs, the doc shows **"Code changed since this doc was last edited"**. A **"Still accurate"** action refreshes the hash without editing the doc. **Docstring changes don't count as code changes.** **v1: functions and methods only** (file and class bodies change too often). **MCP:** `node_context` returns `stale: true`. | new; normalisation as ladder step 4 (VS-D44), plus docstrings | 02-04 §4, 02-03 doc panel |

## 8. Sweep list for VS-D76 / VS-D77 (hand to the other Claude, after the rows are in the log)

Same rule: home docs hold the full rule, summary docs only link.
- `02-technology/05-identity-and-archive.md` §4: add **step 0** above the ladder table (the principle, range tracking, the disk-reload rule, whole-node edits, pending vs saved moves); the `onDidRenameFiles` bullet becomes authoritative, not a hint.
- `06-core/05-sync-and-matching.md`: observed moves applied first at sync; `08-extension/03-documents-and-saving.md`: forwarding incremental `didChange` and rename events to the core, and the reload test.
- `02-technology/04-storage-and-git.md` §4: the body-hash field in doc frontmatter; `02-technology/03-frontend-webview.md`: the "Code changed since…" label and "Still accurate" in the doc panel.
- `00-requirements/03-functional-requirements.md`: one FR each (next free numbers); the user journey: an in-editor rename + rewrite is now found exactly (step 0), not by the ladder.
- Summary docs (README, roadmap): one line each linking VS-D76 / VS-D77. MCP `stale: true` stays in the row only (phase 4).
- Milestone: not decided. Step 0 likely belongs with M4 (docs survive), freshness with M3 or M9. Ask before writing it.

## 9. S4 findings: gix lookups, C9, step 0 checks (interim; paste into `05-spikes/04-git-and-matching.md` when final)

*Status: the gix lookups and the Rust side of C9 are done. The `vscode.git` measurements are done but inconsistent between runs. The reload test and the formatter check are **not finished** (see "Not finished" at the end).*

**gix, the three VS-D74 lookups (done).** `BranchLookups` (repo exists, path in HEAD's tree, current branch, local branches) is implemented by the git binary and by gix behind one trait; the old ancestry and branch-tip methods were removed. Both backends **agree** in every tested situation: a normal repo, a folder and a nested path, a branch named `feature/x`, a detached HEAD, an unborn branch, no repository, a **shallow clone**, and in the middle of a rebase. **No operation needed the git-binary fallback.** Speed on LightRAG (a shallow clone, 112 MB `.git`), median of 30 calls, each gix call re-opening the repository:
| lookup | git binary | gix |
|---|---|---|
| is a repository | 3.4 ms | 0.20 ms |
| path in HEAD (present / absent) | 3.7 / 3.6 ms | 0.46 / 0.46 ms |
| current branch | 3.2 ms | 0.21 ms |
| local branches | 3.7 ms | 0.22 ms |
gix is 7-17x faster. VS-D74's rule is a pure function over these lookups (`reconcile::node_state`), with a table test for each branch of the rule and real-git tests for a checkout, a deleted last-seen branch, and a node file removed from HEAD.

**C9, operations in progress (done on the Rust side).** `rebase-merge` / `rebase-apply`, `MERGE_HEAD` and `CHERRY_PICK_HEAD` in the git directory are detected for all three operations and clear after `--abort`. **During a rebase HEAD is detached** (no current branch, both backends); during a merge or cherry-pick it is not. The pause is a small state machine: wait while an operation runs, reconcile **once** when it ends (unit-tested).

**C9, `vscode.git` (measured; not reliable).** With `--disable-extensions`, the built-in git extension is still present and finds the repository.
- **First run:** a checkout nudged (3 events, first at 116 ms); a merge conflict showed `mergeChanges`; a rebase conflict produced **no event** and `rebaseCommit` was **never reported**; **no abort** (rebase, merge, cherry-pick) produced an event in 1.5 s, and the state stayed stale.
- **Second run, same steps, 5 s window: zero events for every scenario, including the checkout.** Run-to-run the behaviour differed. I did not find the cause (a guess, untested: it refreshes only when the window has focus).
- A **plain file watcher on `.git`** saw every one of them, in both runs: `HEAD`, `rebase-merge` created and deleted (about 23 events during a conflicting rebase), `MERGE_HEAD`, `CHERRY_PICK_HEAD`.
- **`git.enabled = false`:** the git API has **0 repositories** and sends no events; the plain watcher still sees the same `.git` events, so it is a working fallback.
- **Consequence:** do not depend on `vscode.git` to nudge or to report an operation in progress. Watch `.git/{HEAD, rebase-merge, rebase-apply, MERGE_HEAD, CHERRY_PICK_HEAD}` ourselves and read the git directory (the Rust check above); treat `vscode.git` as an optional extra.

**Step 0, the reload test (observations; the harness needs a fix before they can be used).** What VS Code sends, per scenario:
| scenario | content-change event | then | real watcher event |
|---|---|---|---|
| an agent writes the open, clean file | 1 change replacing the text, `isDirty` false, text equals disk | – | yes, within ~5 ms of the change |
| `git checkout` changes it | same | – | yes |
| an agent writes a file with unsaved edits | **none**: the buffer keeps the user's text, still dirty, differs from disk | – | yes (late, ~400 ms) |
| type, then undo to the saved state | `reason: Undo`, `isDirty` still **true** inside the event | a separate zero-change event flips `isDirty` to false | none |
| type, then the user saves | no content change at the save | `onDidSave`; the watcher fires ~80 ms **after** the save (VS Code's own write) | yes, but caused by the save |
| type, then "Revert File" | a change that removes the typed text, `isDirty` false, text equals disk | – | none |
| whole-document replace, in the editor | **1** change over the whole range | dirty flip | none |
| the same through `WorkspaceEdit` | **3 small changes** (VS Code turned the whole-range replace into minimal edits) | dirty flip | none |
Two things to carry into VS-D76: **(1) `isDirty` inside a content-change event is the state from before the change** (typing: false, then a separate zero-change event makes it true), so "not dirty" must be read after that follow-up event; **(2) a revert looks like a reload at the document level** and differs from one only by having no watcher event. The three-part rule as written classified an undo, a save and a revert as reloads in my first run, but that run was contaminated: **my own setup writes produced stale watcher events** inside the observation window. Not yet re-measured cleanly.

**Not finished (waiting for your go-ahead on three small fixes to my own test code):** (1) the reload harness: let the setup writes settle, and judge only content-changing events, after the dirty flip; (2) the formatter check: **the first measurement is invalid** (all four formatters, Black, Ruff, autopep8 and yapf, returned identical edits, so the same provider ran each time, and VS Code seems to post-process formatter edits into minimal ones before an extension sees them). It needs to measure the `didChange` events that arrive after "Format Document" with each formatter, and to verify which formatter ran by comparing outputs. (3) one of my scenario tests (a branch called `feature/x` next to a branch `feature`, which git forbids).
