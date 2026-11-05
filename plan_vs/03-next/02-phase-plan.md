# 03.02: Phase plan

```
PHASE 1  requirements + decisions        ◄── we are here (6 review rounds done)
   │      done when 01-review-checklist A+B are all answered
   ▼
PHASE 2  spikes, then detailed design
   │      spikes answer checklist C; design docs go in new plan_vs/ folders (still one level deep)
   ▼
PHASE 3  build the MVP, milestone by milestone (each one usable on its own)
   │
   ▼
PHASE 4  public pre-release → downloadable drivers → integrations (MCP / CLI) → more languages
```

---

## Phase 2a: four spikes (throwaway code, ~1–2 days each)

They double as a **VS Code extension crash course**: each one teaches one piece end to end.

| Spike | Build | Learn | Answers |
|---|---|---|---|
| **S1: hello core** | An extension command that spawns a Rust binary, sends `vnoc/ping`, and shows the reply in a webview with one xyflow node. Declare `untrustedWorkspaces: limited`. | `package.json`, activation, `vscode-languageclient`, the webview bridge, CSP, F5 debugging, Workspace Trust | B5, C3 |
| **S2: Python driver process** | `vnoc-driver-python` as its own binary; the core spawns it from a drivers folder (copy-on-first-run, VS-D30), handshake, `outline` + `callTree{depth:3}` on a real repo, timed. | the driver protocol, the pylspt actor (plan/04-02), stdout hygiene | C1, C2, C4, C11, C12 |
| **S3: edits without disk writes** | Add/Remove ID via `WorkspaceEdit` on a dirty buffer; save-time IDs via `onWillSaveTextDocument.waitUntil` for Manual/FocusOut (not AfterDelay), skipping functions with syntax errors; the no-loop check; Remove → trash with the `.vnoc/local/trash/` fallback; pylspt's `plan_ids` fed with hash-derived IDs; exact-span removal, including the mismatch case (text edited since the read); slice editing from a webview editor while a regular editor has the same file open. | `WorkspaceEdit`, undo, save participants, `workspace.fs` | B8, C8, C10, C16, C17, C20, C21 |
| **S4: HEAD awareness** | `vscode.git` nudge → Rust reads HEAD with gix, stores `last_synced_head`, runs the **ancestry test**. Cases: checkout (no prompt), pull of a teammate's deletion (Missing), rebase over an upstream deletion (not on this branch → promoted to Missing when the list is opened), all of it again with VS Code closed. Plus the ladder on real refactors (folder rename, move + edit, function rename) and on a fresh / shallow clone, to set the similarity threshold. | the git extension API, gix | B6, C9, C14, C19, VS-O10 |

## Phase 2b: design docs to write

| Folder | Files (first idea) | Builds on |
|---|---|---|
| `04-protocol/` | editor ↔ core messages; core ↔ driver messages + version handshake; errors; cancellation; TS generation | 02-02, 02-07, plan/06-api/03 |
| `05-rust-core/` | crate layout, domain types (Anchor, Side, IdMode…), SQLite schema, `vnoc-userdata` (two sides + placement rules), scheduler (auto/manual, no watcher), HEAD-aware reconcile | plan/01, 03, 05 |
| `06-identity-impl/` | comment placement rules, `id_in_code`, `[[path]]` resolution, exact-match removal, the six-step ladder, two-IDs merge, duplicate IDs, save-time edits, tests | 02-05, plan/04-04 |
| `07-call-tree-lazy/` | layer 1 / layer 2 on demand, `expand`, pulled-in files, hash-checked expansions | plan/05-07 |
| `08-drivers/` | driver host: lookup, install, manifest, minisign, lock, GC, proxy, offline install; the Python driver wrapper | 02-07 |
| `09-extension/` | `package.json` contributions, commands/menus, TreeView, CodeLens, setup QuickPicks, trust, git API, save participant | 00-02 |
| `10-webview/` | app structure, store (focus stack), canvas + call-tree components, port list from old v-noc | 02-03 |
| `11-release/` | CI (`release-core`, `release-driver`), signing keys, packaging, stores, test strategy | 02-06, plan/07-02 |

## Phase 3: MVP milestones

| M | Delivers | The user can… | Reuses from `plan/` |
|---|---|---|---|
| **M0** | Monorepo skeleton: core crates, `drivers/python`, extension, webview; CI builds a VSIX | install it | plan/02-01 (changed) |
| **M1** | Driver host (copy-on-first-run + lookup + handshake), structure index of the scope, SQLite cache, sidebar tree, **Quick look**, Go to code, Restricted Mode | right-click ▸ Quick look ▸ browse | plan M1, M4, M5 |
| **M2** | **Canvas + focus stack** with the old v-noc node card (expand, code toggle, zoom dialog) and Canvas \| Code tabs, Reveal in V-NOC, auto sync from VS Code events, manual sync | explore visually | plan/05-09 scheduler |
| **M3** | Setup (scope, team/solo, edit code?), `vnoc.toml` + `local.toml`, **shared/private docs**, `node.toml` + sharded node folders (`none` mode), the two-IDs merge, the six-step ladder, **HEAD awareness**, Missing / Not on this branch / Archive / Relink / Remove, Share / Make private | document code, keep docs across refactors and branches, share through git | plan/05-06 |
| **M4** | **Lazy call trees** (depth setting, expand), pulled-in files, markers | open any function's call tree | plan M7–M8 |
| ⭐ | **First public pre-release** (Marketplace + Open VSX) | | |
| **M5** | **In-code IDs**: `on-attach` / `all` + `[[path]]` rules, per-node Add / exact-match Remove, Restore / Keep removed, previews, save-time IDs | choose how permanent IDs are | plan/04-04 |
| **M6** | Playground | try a function on its own | |
| **M7** | CodeLens, hover preview, search, `vnoc driver install <file>` | polish | |

**Why IDs come after docs (M5 after M3):** `none` mode must work on its own (it's the hardest matching case), and solo users default to it. Building it first means
the matching ladder and HEAD handling are solid before in-code IDs make most cases easy. It also lets the first release go out with zero edits to anyone's code.

## Phase 4: after the MVP

Driver **downloads** turned on (signed manifest, VS-D29). The lookup path already exists from M1, so this only adds a new source → TS/JS driver → `vnoc mcp` + `vnoc query`
(Claude Code, Cursor agent, opencode, pi) → background call-tree prefetch (VS-O8) → groups → docs on call nodes → logs / tasks / history.
