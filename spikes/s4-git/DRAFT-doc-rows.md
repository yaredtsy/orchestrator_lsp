# Draft rows for the docs you maintain (not applied)

Written 2026-11-06 during spike S4. Nothing here has been put into `01-decision-log.md`, `01-review-checklist.md` or `05-spikes/`. Every draft is in this one file: §0 VS-D74, §1 VS-D75, §2 the VS-D73 amendment, §3 checklist, §4 findings, §5 sweep (VS-D75), §6 VS-D76, §7 VS-D77, §8 sweep (VS-D76/D77), §9 findings for the remaining S4 checks, §10 closing summary, §11 roadmap edits for the MVP cut, §12 VS-D78 (the MVP cut), §13 VS-D79 (the node ID format) with its sweep, §14 the final MVP file formats, §15 VS-D80 (the driver protocol decisions).

## 0. Decision log: row VS-D74 (your original text)

Your text, with the line breaks joined so it fits one table row. Wording unchanged.

| VS-D74 | Missing vs not on this branch is decided by where node.toml lives, not by how HEAD moved. When a function is absent after the ladder, check in order: status = archived -> Archived. No git repository (gix finds none) -> Missing. node.toml committed in HEAD's tree -> Missing (orphaned in this commit, works on shallow clones). Otherwise the node floats: no last_seen(id, branch) record -> not on this branch; HEAD on the last-seen branch -> Missing; last-seen branch still exists -> not on this branch; last-seen branch deleted -> Missing. last_seen updates only when the node is present and HEAD is on a branch. Detached HEAD never updates it, counts as another branch, and shows no Missing notice. A renamed branch looks deleted (accepted). Anchor rewrites after a found move: committed node.toml only for moves in uncommitted working-tree changes. Moves already committed in HEAD stay in SQLite with an "Update anchors" action. Floating node.toml only when HEAD is on its last-seen branch. Never on detached HEAD. C9 pause stays, for speed only. Duplicate in-code IDs: VS-D71's order, then folder exists -> ask (R9), else first by path, then line. | supersedes VS-D34, VS-D72 (2) · refines VS-D71 | 02-05 §2, §5 · 02-04 :170 |

**How my earlier rebuilt row differed from yours** (I kept yours; this is only so you can see what a reconstruction from the docs misses):
1. **The duplicate in-code ID clause was missing from mine** ("VS-D71's order, then folder exists -> ask (R9), else first by path, then line"), and with it the **"refines VS-D71"** and the **`02-05` §2** reference. This was the one real difference in meaning.
2. Mine named the three gix lookups (path in HEAD's tree, current branch, local branches). Yours doesn't; they are in the S4 findings instead.
3. Mine said "(replaces the ancestry test of VS-D34 and the ref check of VS-D72)" and "refines VS-D45"; yours says "supersedes VS-D34, VS-D72 (2)" and does not mention VS-D45.
4. Mine lacked "(gix finds none)" after "No git repository".
Everything else (the four-way floating rule, `last_seen`, detached HEAD, the anchor-rewrite cases, "Update anchors", the pause) is the same in meaning.

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
- Milestone: step 0 is post-MVP and ships with no-ID mode and ladder steps 2-6 (VS-D78, §11). Freshness stays Stable / M9.

## 9. S4 findings: gix lookups, C9, step 0 checks (paste into `05-spikes/04-git-and-matching.md`)

**gix, the three VS-D74 lookups: done, no fallback.** `BranchLookups` (repo exists, path in HEAD's tree, current branch, local branches) is implemented by the git binary and by gix behind one trait; the old ancestry and branch-tip methods were removed. **Both backends agree in every tested state:** a normal repo, a folder and a nested path, a branch named `topic/x`, a detached HEAD, an unborn branch, no repository, a **shallow clone**, and in the middle of a rebase. **No operation needed the git-binary fallback.** Speed on LightRAG (a shallow clone, 112 MB `.git`), median of 30 calls, each gix call re-opening the repository:
| lookup | git binary | gix |
|---|---|---|
| is a repository | 3.4 ms | 0.20 ms |
| path in HEAD (present / absent) | 3.7 / 3.6 ms | 0.46 / 0.46 ms |
| current branch | 3.2 ms | 0.21 ms |
| local branches | 3.7 ms | 0.22 ms |
**gix is 7-17x faster.** VS-D74's rule is a pure function over these lookups (`reconcile::node_state`), with a table test for each branch of the rule and real-git tests (a checkout, a deleted last-seen branch, a node file removed from HEAD).

**C9, the pause: done on the Rust side.** `rebase-merge` / `rebase-apply`, `MERGE_HEAD` and `CHERRY_PICK_HEAD` in the git directory are detected for rebase, merge and cherry-pick, and clear after `--abort`. **During a rebase HEAD is detached** (no current branch, both backends); during a merge or a cherry-pick it is not. The pause is a small state machine: wait while an operation runs, reconcile **once** when it ends (unit-tested).

**C9, `vscode.git`: unreliable, cause unknown. Decision: the core watches `.git` itself for the nudge; `vscode.git` is not used.** What was measured (built-in git extension present even with `--disable-extensions`): first run, a checkout nudged (3 events, first at 116 ms), a rebase conflict produced no event and `rebaseCommit` was never reported, and no abort produced an event in 1.5 s; second run, same steps, 5 s window: **zero events for every scenario, including the checkout.** A plain file watcher on `.git` saw all of it in both runs: `HEAD`, `rebase-merge` created and deleted (about 23 events during a conflicting rebase), `MERGE_HEAD`, `CHERRY_PICK_HEAD`. With `git.enabled = false` the git API has **0 repositories** and no events, while the plain watcher is unaffected.

**Step 0 checks: post-MVP, unfinished. Both reopen when step 0 is built.**
- **Reload test: the harness was contaminated** by stale watcher events from its own setup writes, so the three-part rule's verdicts from that run cannot be used. **Known facts:** `isDirty` inside a content-change event is the state from **before** the change (typing: false in the change event, true in a separate zero-change event right after), and a **"Revert File" looks like a reload at the document level** (not dirty, text equals disk) and differs only by having no watcher event. Also: an agent write to a file with unsaved edits produces **no content change** (the buffer keeps the user's text); a whole-range replace in the editor arrives as **one** change, and the same through `WorkspaceEdit` arrives as **three small** ones (VS Code minimises it); a user save fires no content change, and the watcher fires about 80 ms after it.
- **Formatter measurement: invalid.** Black, Ruff, autopep8 and yapf all returned identical edits, so the same provider ran each time; and VS Code seems to post-process formatter edits into minimal ones before an extension sees them. Needs measuring through the `didChange` events after "Format Document", checking which formatter ran.

## 10. S4 closing summary (draft)

**Done and tested** (15 unit tests, 10 real-git scenario tests, the fixtures, the tuning): the ladder with its three tiers; twins and short bodies ask only; VS-D74's rule on the gix lookups; the pause for rebase, merge and cherry-pick.
- **Fixtures:** the six refactors (+ two extras) behave as expected, with no wrong silent match.
- **VS-O10 (the rule):** silent only at an Edit score of at least 0.99 (Jaccard at least 0.91), zero wrong silent matches, validated on repos it was not tuned on. **In practice only an edit-free rename or move is matched silently; anything edited is asked.**
- **Provisional tier:** built, off in v1 (VS-D75); no floor reaches 95% on the synthetic set.
- **gix:** the three lookups agree with the git binary everywhere and are 7-17x faster; no fallback.
- **C9:** the pause is built; the core watches `.git` itself, `vscode.git` is not used.
- **MVP gating:** ladder steps 2-5 and the provisional tier are behind `Config::steps` (`Steps::mvp()` = step 1 only, the default); 22 unit tests include the MVP behaviour. With step 1 only, every moved or renamed node on the fixtures becomes Missing and none becomes a wrong match.
- **Post-MVP:** the step 0 checks (reload test, formatters).

**Pass / fail of the spike:** branch switches don't prompt and a pulled deletion does (VS-D74 rule, tested with real git); zero wrong silent matches on the fixtures and the labelled set, with a threshold chosen and validated; B6, C9 and C14 (reshaped by VS-D72/D74 to "gix lookups, no ancestry") answered.

## 11. Roadmap edits for the MVP cut (apply to `04-roadmap/`; the doc session does it)

The cut reverses several statements in both roadmap docs. Edits are in the order of the files; the answers to the earlier open questions are built in. **What is still unassigned** is at the end.

### `01-mvp-vs-stable.md`
- **Release table, MVP "one job":** replace "Never edit the user's code by itself" with the promise: **"V-NOC edits your code only to write one ID line when you attach a doc. The edit goes through VS Code's editor and can be undone."** **Stable row:** "in-code IDs" leaves the list; stable becomes "the post-MVP P-list (below), playground, polish, hardening".
- **Area table:**
  - *Setup:* MVP = Quick look; guided setup for **scope only**; a minimal `vnoc.toml` (scope), hand-written + reload. Moves to the P-list: team / solo and `local.toml` (**P2**), the settings UI, `[[path]]` rules, monorepo overrides, effective-config view and package detection (**P3**), the no-ID mode (**P1**) and the `all` mode (**P3**).
  - *Docs:* MVP = doc panel (read, edit, add, remove); doc folders `nodes/<shard>/<id>/` with `node.toml` and any number of `.md` files; **no private docs, no Share / Make private**. Moves to the P-list: private docs and Share / Make private (**P2**), doc freshness (VS-D77, **P4**).
  - *Identity:* MVP = **ID mode only**. **IDs are written into code only when something is attached to a node** (a doc, in the MVP; tagging every function, the `all` mode, stays **P3** as an explicit setup action). The ID is a docstring `ID:` line, written through the driver's `plan_ids` and a `WorkspaceEdit`); a lost ID comment is **restored when a function with the same `path::qualname` still exists** (ladder step 1), otherwise the node is Missing. **Exception that is in the MVP: `onWillRenameFiles` / `onDidRenameFiles` update file and folder anchors exactly** (old path to new path; a prefix change for a folder). **Terminal and git renames go to Missing, relink by hand.** Out of the MVP: Remove ID and `# FileID:` (**P4**), Keep removed (**P1**), save-time IDs (**P3**).
  - *Lost nodes:* MVP = Missing / Not on this branch (VS-D74 as built; uncommitted docs float; the duplicate in-code ID rule stays), **relink by hand**, **archive (no expiry)**, **remove to the V-NOC trash (VS-D70)**, and **two command-palette commands: "V-NOC: Empty trash"** (the only permanent delete, with a confirmation) **and "V-NOC: Restore removed doc"** (a quick pick over `trashed.toml` that moves the files back). **Relink by hand moves the source folder's docs into the target node's folder** (a suffix on a file name that clashes). The **strong two-ID merge is out** of the MVP. **The "Recently removed" panel is P4**; the MVP restores with the command. Weak-match prompts are P1.
- **"Why this cut" (the first two bullets argue the opposite and must be rewritten):** (1) *was* "MVP never writes to the user's code", (2) *was* "`none` mode is the hardest case, so it goes first". **New reasons:** IDs are always in code, so a rename or a move **carries the ID** and the ladder is not needed to follow renames (step 1 only restores a lost comment); VS Code's own rename events cover renames done in the editor; the one code write is small, explicit and undoable; the **on-disk format is final from the MVP**, so nothing after it needs a migration. (Bullets 3 and 4, canvas editing and the playground, stay.)
- **"The promise that starts at the MVP: the data format":** the `vnoc.toml` row becomes "`vnoc.toml` keys that exist in the MVP (scope only, `version = 1`)"; add **"The on-disk format is final from the MVP on: folder layout, ID format, `node.toml` fields. Post-MVP features must not need a migration."**
- **MVP exit criteria:**
  - **2 (the refactor suite)** is reworded: docs survive rename-function, move-file, rename-folder, move-and-edit and cut/paste between files **because the ID comment moves with the function**; a rename or move done in VS Code updates the anchors exactly; a lost comment with the function still at its `path::qualname` is restored; a **terminal or git rename ends in Missing → relink by hand**, never in a wrong attachment. The S4 refactor table (steps 2-6) is the **P1 suite**, not an MVP gate.
  - **3** (branch switches, VS-D74): unchanged.
  - **4 (no data loss):** drop "Make private / Share / two-ID merge" (post-MVP / out); keep Remove and the V-NOC trash, add **relink by hand** (docs moved into the target folder, nothing overwritten) and the **Empty V-NOC trash** command. "Every lost file is recoverable" holds in the MVP: git, or **"V-NOC: Restore removed doc"** for files in the trash. Only the Recently-removed panel is P4.
- **Stable exit criteria:** criterion 1 ("in-code IDs pass a two-person team test") becomes the criteria of the P-list items that ship: **P1 passes the refactor suite with zero wrong silent matches**, plus a two-person team test for the shared formats.

### `02-milestones.md`
- **M3:** "guided setup, `vnoc.toml` / `local.toml`, `.gitignore` per mode, `node.toml`, doc panel (read, add, remove), shared / private, Share / Make private" becomes: **guided setup (scope only), minimal `vnoc.toml`, `node.toml`, the doc folders, the doc panel (read, add, remove), and writing the hash ID into code on attach (driver `plan_ids` → `WorkspaceEdit`).** The ID write is **M3's**, with the doc panel. **M6 keeps slice editing.**
- **M4:** "step 0 (observed moves, VS-D76) + matching ladder, Missing / Not on this branch (VS-D74) / Archive / Relink / Remove, V-NOC trash + Restore, strong two-ID merge" becomes: **Missing / Not on this branch (VS-D74), relink by hand, archive (no expiry), remove to the V-NOC trash (VS-D70) with the Empty V-NOC trash command, restore a lost ID comment (step 1), and the `onWillRenameFiles` / `onDidRenameFiles` anchor updates.** Step 0 (beyond those rename events), ladder steps 2-6 and the two-ID merge leave M4.
- **"Why this order":** replace "**Step 0 ships in the same milestone as the ladder (M4), never after it** (VS-D76)" with **"Step 0 ships together with no-ID mode and ladder steps 2-6, never after them (VS-D78)."** Record the reason beside it: "never after it" protected no-ID mode, where the ladder carries renames; in the MVP the ID carries them.
- **Post-MVP is a P-list, not milestone numbers.** Replace the **Stable milestones M7 (In-code IDs)** and the setup / polish items of **M9** with a section "Post-MVP (P-list; the order is set after tester feedback)":
  | | Items |
  |---|---|
  | **P1** | no-ID mode + ladder steps 2-6 (with the twin and short-body rules) + step 0 (VS-D76), **together**; **Keep removed** (a documented node with no in-code ID needs no-ID matching); **weak-match prompts** (they come from ladder steps 2-5) |
  | **P2** | private docs, `local.toml`, solo mode, **Share / Make private** |
  | **P3** | settings UI, `[[path]]` rules, monorepo overrides, **`all` mode**, **save-time IDs** (they serve `all` mode) |
  | **P4** | archive expiry, doc freshness (VS-D77), the provisional tier (VS-D75), Remove ID, **`# FileID:`**, the **"Recently removed" panel** |
  M8 (Playground), M10 (Hardening) and the rest of M9 (CodeLens, hover, search, `vnoc driver install`) are not changed by the cut. **M9 loses** archive expiry, the "Empty V-NOC trash" UI (now an MVP command and the P4 panel), doc freshness and weak-match prompts, all of which are in the P-list.
- **"Done" for every milestone:** the test list mentions the ladder; in the MVP it covers **step 1, the VS-D74 rule, and the rename-event anchor updates**.

### Follow-ups outside the roadmap (the doc session should also check)
- The **M / S column** in `00-requirements/03-functional-requirements.md`: every FR for private docs, solo mode, `local.toml`, the settings UI, `[[path]]` rules, no-ID mode, step 0 (except the rename events), the ladder steps 2-6, the two-ID merge, Remove ID, Keep removed, save-time IDs, `# FileID:`, weak-match prompts, Share / Make private and the "Recently removed" panel moves out of the MVP. **FR-75 and its Restore part (VS-D70) must say that the "Recently removed" panel is P4 and that the MVP has two commands: Empty trash and Restore removed doc.**
- `00-requirements/02-user-journey.md` and `00-requirements/05-user-controls-and-config.md`: the `none` / solo defaults (VS-D25) are post-MVP.
- VS-D76's own row says file renames are applied from `onWillRenameFiles` / `onDidRenameFiles`: that part is the **MVP exception** in VS-D78; the rest of VS-D76 is P1.

### Nothing is left unassigned
Every item of the cut now has an MVP place or a P number (P1-P4 above).

## 12. Decision log: new row VS-D78 (the MVP cut)

| VS-D78 | **[you] The MVP cut.** **Promise:** "V-NOC edits your code only to write one ID line when you attach a doc. The edit goes through VS Code's editor and can be undone." **MVP storage:** **(1)** `.vnoc/` at the git root; a minimal `vnoc.toml` (scope only). **(2)** Doc folders `nodes/<shard>/<id>/` with `node.toml` and any number of `.md` files. **(3)** **IDs are written into code only when something is attached to a node (M3, with the doc panel: driver `plan_ids` → `WorkspaceEdit`); ID mode is the only mode; tagging every function (`all` mode) stays P3, as an explicit setup action.** **(4)** SQLite cache. **(5)** Missing and not-on-this-branch: **VS-D74 as built** (uncommitted docs float; the duplicate in-code ID rule stays); actions: **relink by hand, archive (no expiry), remove to the V-NOC trash (VS-D70)**. **Relink by hand moves the source folder's docs into the target node's folder** (a suffix on a file name that clashes). **(6)** **Lost ID comment:** if a function with the same `path::qualname` exists, offer to restore the ID (ladder step 1); otherwise Missing. **(7)** **`onWillRenameFiles` / `onDidRenameFiles` update file and folder anchors exactly** (old → new path; a prefix for a folder); **terminal and git renames → Missing, relink by hand.** **(8)** **Two command-palette commands for the trash: "V-NOC: Empty trash" and "V-NOC: Restore removed doc"** (a quick pick over `trashed.toml` that moves the files back, never overwriting); the "Recently removed" **panel** stays P4. **(9)** The C9 pause, the `.git` watcher, `WorkspaceEdit` for edits, Workspace Trust and the stdout redirect: as built. **Out of the MVP:** Remove ID, Keep removed, save-time IDs, `# FileID:`, the **strong two-ID merge**, the Recently-removed panel. **Post-MVP (designed and recorded, not built), as a P-list whose order is set after tester feedback:** **P1** no-ID mode + ladder steps 2-6 (with the twin and short-body rules) + step 0 (VS-D76), together, plus Keep removed and weak-match prompts; **P2** private docs, `local.toml`, solo mode, Share / Make private; **P3** settings UI, `[[path]]` rules, monorepo overrides, `all` mode, save-time IDs; **P4** archive expiry, doc freshness (VS-D77), the provisional tier (VS-D75, already off), Remove ID, `# FileID:`, the "Recently removed" panel. **New rule for step 0 (replaces "never after the ladder" in VS-D76): step 0 ships together with no-ID mode and ladder steps 2-6, never after them.** Why: "never after it" protected no-ID mode, where the ladder has to carry renames; in the MVP IDs are always in code, so a rename carries the ID and the ladder only restores a lost comment. **The on-disk format is final from the MVP on** (folder layout, ID format, `node.toml` fields): post-MVP features must not need a migration. In the spike code the MVP path uses ladder step 1 only; steps 2-6 and the provisional tier stay behind config, off (`Config::steps`). | refines VS-D76 (when step 0 ships; its rename events are the MVP exception), VS-D75, VS-D74, VS-D44; supersedes the two-ID merge (VS-D40) for the MVP | 04-roadmap/01, 02 |

## 13. Decision log: new row VS-D79 (the node ID format) and its sweep

The reference implementation is `crates/vnoc-domain` (`src/id.rs`, with the spec in its doc comments). Its tests compare against **golden values written once by an independent oracle** (`tests/golden/gen_golden.py`: Python `blake3` + a hand-written Crockford encoder, qualnames from Python's own compiler).

| VS-D79 | **[you] The node ID format, final from the MVP on.** **Name:** `NodeId`, **not a ULID**: a **128-bit deterministic node ID** written in Crockford base32. **Making one:** BLAKE3 **`derive_key`** with the context string **`"v-noc node id v1"`**; the input is the UTF-8 bytes of **`repo_path` `\0` `qualname` `\0` `occurrence`** (occurrence in decimal); on a **collision** (VS-D71's check) append **`\0` and a counter in decimal, starting at 2**, and re-hash until the ID is free; the output is the **first 16 bytes read as a big-endian 128-bit integer**. **Writing one:** Crockford base32, **26 characters, uppercase**, the two spare bits in front so the **first character is 0-7**; **reading is case-insensitive** (`I` and `L` read as `1`, `O` reads as `0`; `U` is rejected); the **shard is the last 2 characters, uppercase**. **The inputs:** `repo_path` is repo-relative with forward slashes **exactly as git's index records it** (precomposed Unicode on macOS: take it from git, never from a directory listing); `qualname` is Python's **`__qualname__`, including `<locals>`**; `occurrence` is **1-based**, counted among definitions with the same `qualname` in the same file, in source order. **In code:** a line **`ID: <id>` in the def's or class's docstring**, nothing else (there is no comment form; a file-level `# FileID:` is P4). **Who may create a `NodeId`:** one exists only if it was **read** (from code, from a `.vnoc/` folder name, from `node.toml`, from doc frontmatter) or made by **`persist()`**, which **writes it into the code before returning it**. A node that has not been persisted is a `LocationKey`. Where a provisional hash is needed it is a **separate type that cannot be stored or serialized**. **Anything that saves a reference to a node (doc links, MCP, saved tabs) calls `persist()` first.** **Old V-NOC IDs (UUIDs) are not supported: that support is dropped.** **When persisting into a docstring that already has an old-format `ID:` line, that line is replaced in place (its exact span); a second `ID:` line is never added.** **Nothing in the product reads IDs made by the spikes** (their encoding and qualnames differ). This is part of the on-disk promise of VS-D78 and cannot change after the MVP. | supersedes VS-D50 (the formula and the collision counter), VS-D60 (the encoding detail), and the comment form of VS-D36; refines VS-D71 (the counter) and VS-D54 | 02-05 §1, §2 · 02-04 §4 |

**Sweep list for VS-D79** (hand to the doc session; home docs hold the full rule, summary docs only link):
- `02-technology/05-identity-and-archive.md` §1: the formula block (the plain `base32(blake3(path + "::" + qualname + "#" + occurrence)[..16])` becomes the `derive_key` rule above); the "Taken already?" counter text (`path::qualname#1#2`, `#3` becomes "append `\0` and a counter from 2"); the "Encode like a ULID … Plain RFC 4648" paragraph (it is now a hand-written Crockford encoder, no `ulid` crate); the **Rust note** `derive_id` and "Reusing `ulid`'s encoder" (replace by a pointer to `crates/vnoc-domain/src/id.rs`); every `PermId` becomes `NodeId`; the table row "Permanent ID".
- `02-technology/05-identity-and-archive.md` §2: the in-code ID is a **docstring line only**; `# FileID:` stays but is P4. **Add the rule:** persisting into a docstring that already has an old-format `ID:` line **replaces that line in place (exact span)** and never adds a second one.
- `02-technology/05-identity-and-archive.md` §1: **delete the bullet** "Old V-NOC IDs (UUID v4 strings, 36 chars) are still read and kept. Sharding by their last 2 hex characters works the same." UUID IDs are no longer read.
- `02-technology/04-storage-and-git.md` §4: "printed like a ULID" and the **Rust note** "why Crockford base32 … The `ulid` crate's encoder does it" (hand-written now); the snippet at lines 117-119 (`&PermId` becomes `&NodeId`).
- `04-roadmap/01-mvp-vs-stable.md` line 58: "(blake3 → 26 chars, ULID-style encoding)" becomes "(`NodeId`: BLAKE3 `derive_key`, Crockford base32, 26 characters)".
- `06-core/01-crates-and-domain.md` (items 2 and 3) and `06-core/06-identity.md` (item 1): `PermId` becomes `NodeId`; "ULID-style encoding" becomes "Crockford base32". The domain types themselves are documented in the code.
- `01-decisions/02-relation-to-old-plan.md` line 25: no change needed.
- The decision-log history rows that say ULID (VS-D38, VS-D41, VS-D50, VS-D60, and the review rows R1, R13) are history: your call whether to annotate them.

**Decided: the old V-NOC IDs (UUIDs) are dropped.** `NodeId` refuses a UUID (a test pins it). The one rule left is the replace-in-place above, which the driver carries out when the core marks the item (`replace_existing`, see VS-D80 and `07-01`).

## 14. The final MVP file formats (draft for `02-04` §4-5 and `00-05`)

**Rules for every file V-NOC owns, final from the MVP on (they are what makes later fields migration-free):**
1. **Readers ignore fields they do not know. Writers keep them.** TOML files are edited in place with `toml_edit`: unknown keys, comments and key order survive a read-modify-write. A doc's frontmatter keeps unknown lines verbatim.
2. **The MVP writes only the fields it uses**, plus `id_in_code = "yes"` (a later mode reads it, and it is free now).
3. A *known* field with a value this version does not know is read as "other" and **kept untouched**.
4. **Only `vnoc.toml` carries a version** (`format = 1`). A file with a `format` newer than the reader supports is **refused, not rewritten**. Adding a field is not a format change.
5. Anything a post-MVP feature needs must be a new optional field, never a changed meaning.

**`.vnoc/vnoc.toml`** (one per repo; the draft in `00-05` used `version = 1`, now `format = 1`):
```toml
#:schema ...                    # optional comment, kept
format = 1                      # the only required key
[index]
include = ["src/**"]            # the scope: where call trees start and where IDs may go. Default ["**"]
exclude = ["**/migrations/**"]  # on top of .gitignore
```
The MVP writes `format`, and `include` / `exclude` only when the user sets them. **Read and kept if present, not written by the MVP:** `[project] sharing`, `[index] on_open / sync / respect_gitignore / languages`, `[ids] default`, `[call_tree]`, `[retention]`, `[[path]]`.

**`.vnoc/nodes/<shard>/<id>/node.toml`** (the code is `crates/vnoc-domain/src/node_toml.rs`, with tests that keep an unknown field, a comment and the key order):
```toml
path       = "src/orders.py"              # anchor: the repo path, as git records it
qualname   = "OrderService.parse_order"   # anchor: Python __qualname__ (with <locals>)
id_in_code = "yes"                        # "yes" | "no" | "never"; the MVP always writes "yes"
status     = "active"                     # "active" | "archived"
# archived_at = 2026-11-06                # a TOML date, written when the node is archived
# occurrence = 2                          # only when it is not 1
```
**Two additions, confirmed:** (a) **`occurrence`**, written only when it is not 1 (absent = 1): the anchor needs it to find the right definition when a function is defined twice in a file, and the lost-ID-comment restore (step 1) uses the anchor; (b) **`archived_at` is written by the MVP** even though nothing expires: archive expiry (P4) needs the date for nodes archived during the MVP.

**Doc frontmatter** (each `.md` in the node folder):
```markdown
---
node: 3RGJ87CMS6TK775XRJV8BW977Y
hint: src/orders.py::OrderService.parse_order
---
```
The MVP writes `node` and `hint`. `hint` is rewritten when the doc is saved and is **never used for lookup**; if `node:` and the folder disagree, **the folder wins** and `node:` is fixed on the next save. `body_hash` (VS-D77) is P4 and is not written by the MVP.

**`.vnoc/.gitignore`**: the MVP writes one line, **`local/`**, because the V-NOC trash lives there (VS-D70: `.vnoc/local/trash/`, never committed). P2 adds `local.toml`. **Confirmed as part of the frozen format.** **V-NOC ensures its own line exists and never removes a line the user added.**

**`.vnoc/local/trash/<timestamp>_<node-id>/trashed.toml`** (VS-D70), read by "V-NOC: Restore removed doc":
```toml
[[file]]
node = "3RGJ87CMS6TK775XRJV8BW977Y"
side = "shared"                          # the MVP has only "shared"; P2 adds "local"
file = "nodes/7Y/3RGJ87CMS6TK775XRJV8BW977Y/why-we-parse-twice.md"   # the path relative to .vnoc/
removed_at = "2026-11-06T09:30:00Z"
```

**Decided:** the call-tree **depth** setting (MVP, M5) is a **VS Code setting**, not a key in the repo's `vnoc.toml` (the cut makes `vnoc.toml` scope-only).

## 15. Decision log: new row VS-D80 (the driver protocol decisions of `07-01`)

All five proposals of `plan_vs/07-drivers/01-driver-protocol.md` are agreed; this records them (the wire itself is documented there, on its own, per VS-D68).

| VS-D80 | **[you] The core ↔ driver wire, protocol 1: five decisions.** **(1) One position convention on the whole wire: 0-based line, UTF-16 `character`** (as LSP and VS Code); **the driver converts ruff's byte offsets to this at its boundary**, so no other component sees a byte offset. **(2) `planIds` is keyed by `(qualname, occurrence)`, not by a position**; **the core picks the ID and says whether an existing `ID:` line is to be replaced (`replace_existing`), the driver picks the place and carries it out**: with the flag it replaces the existing line in place (exact span), without it a def that already has an ID gets no edit. **(3) `outline` is one flat request** (defs in source order, parents before children) with optional `fingerprints` (P1). **(4) `environmentInputs` returns whole files to hash**, accepted for the MVP: a change only clears call-resolution data (the structure is unaffected); **the narrower trigger (only the `pyproject.toml` tables that hold dependencies) is post-MVP (D4).** **(5) The qualname rule is the driver's job** (Python: `__qualname__`, including `<locals>`), since it is language-specific; the core treats a qualname as opaque and counts occurrences itself. | refines VS-D68, VS-D53; supports VS-D79 | 07-01 |
