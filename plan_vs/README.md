# V-NOC for VS Code: Plan 2 (`plan_vs/`)

> **What this is:** the plan for shipping V-NOC as a **VS Code extension** (also Cursor, Windsurf, VSCodium). The Rust core runs next to the
> editor, keeps an embedded SQLite cache inside the project's `.vnoc/`, and talks to **language drivers** (separate executables; Python first).
>
> **How it relates to [`../plan/`](../plan/README.md):** that plan was for a server backend (TerminusDB + gRPC + a web app). This plan
> **diverges** where the product changed (storage, API, packaging, identity, drivers) and **points back** to `plan/` where the Rust core
> stays the same (scan, diff, reconcile, call-tree layers). See [`01-decisions/02-relation-to-old-plan.md`](01-decisions/02-relation-to-old-plan.md).
>
> **Phase:** 🟢 **Phase 1: requirements and decisions.** Six review rounds are in the [decision log](01-decisions/01-decision-log.md).
> Phase 2 starts once [`03-next/01-review-checklist.md`](03-next/01-review-checklist.md) is answered.
>
> **Style:** written to teach. New VS Code ideas get a **🧩 VS Code note**, Rust ideas a **🦀 Rust note**.

---

## The whole product as one tree: what the user does

Each letter has its own section in [`00-requirements/02-user-journey.md`](00-requirements/02-user-journey.md).

```
User installs "V-NOC" (VS Code Marketplace, or Open VSX for Cursor / Windsurf / VSCodium)
│   ships with: the core + the Python driver (copied on first run into the global drivers folder)
│
├── A. OPENS A FOLDER
│   ├── A1. .vnoc/vnoc.toml exists ──► index on open (if configured) ──► C
│   │     ├── untrusted workspace ──► read-only: index + browse; no code edits, no playground runs
│   │     └── HEAD moved while VS Code was closed ──► same ancestry test as E4/E5
│   ├── A2. no .vnoc/ ──► stay quiet; the V-NOC view offers "Set up" / "Quick look"
│   └── A3. a needed language driver is missing ──► "Install the X driver?" (always the user's choice)
│
├── B. SETS UP A PROJECT  (.vnoc/ always at the git root; what you click = the SCOPE)
│   ├── B1. Quick look        right-click ▸ "V-NOC: Quick look"
│   │     │                   no questions; cache in VS Code's own storage; NOTHING in the repo
│   │     └── "Make it a project" ──► creates .vnoc/ ──► B2
│   ├── B2. Guided setup      right-click ▸ "V-NOC: Set up here"  — asks only what YOU know:
│   │     ├── Q1 What to index?                        (the scope)
│   │     ├── Q2 Who sees the notes?    ○ My team (commit .vnoc/)   ○ Just me (local side only, out of git)
│   │     └── Q3 ID comments in your code?
│   │            ○ On attach (team default)  ○ All  ○ None (solo default)   + per-folder rules later
│   └── B3. Hand-written      .vnoc/vnoc.toml (team) and/or .vnoc/local.toml (just me) ──► "Reload"
│
├── C. EXPLORES
│   ├── C1. Sidebar tree      Root ▸ folders ▸ files ▸ classes ▸ functions  (pulled-in files dimmed)
│   ├── C2. Canvas            open any node as a graph; double-click = focus
│   │     ├── node card       [▸] expand · [</>] code inside the card · [⤢] zoom dialog · 🔗 ID · 📄 docs
│   │     ├── tabs            Canvas | Code   (code editable: a view of VS Code's document)
│   │     └── ★ Focus stack   focus · Back · Clear · click the breadcrumb to jump
│   ├── C3. Call tree         lazy only: open ▸ N levels ▸ expand ▸ N more   (N = a setting, default 3)
│   │     ├── calls leaving the scope are still followed; those repo files get pulled in, never ID'd
│   │     └── third-party calls (site-packages, .venv, node_modules, outside repo) = external leaves (open read-only)
│   ├── C4. Jump to code      the editor opens at the right lines (code is never copied)
│   └── C5. Reveal in V-NOC   from the editor cursor
│
├── D. ADDS KNOWLEDGE
│   ├── D1. Attach a doc      ○ Shared (team sees it)  ○ Private 🔒 (only me)  → a .md file, normal editor tab
│   │     └── later: "Share this doc" / "Make private"
│   ├── D2. Playground        a snippet bound to a node, run in the VS Code terminal (trusted only)
│   └── D3. Per-node ID       right-click any file / class / function (in scope)
│         ├── [Add permanent ID]     ──► one `ID:` line, placed by the driver (Python: docstring); an undoable edit
│         ├── [Remove permanent ID]  ──► the recorded span, only if its text still matches; docs stay; id_in_code = never
│         └── folder ▸ IDs for this folder and below ▸ All / On attach / None / Inherit
│
├── E. KEEPS CODING (background)
│   ├── E1. auto sync:   edit · save · rename · move ──► small re-index
│   │   manual sync: "V-NOC: 12 changed · Sync"  (call trees stay correct anyway)
│   ├── E2. `all` rule: new functions get their ID inside YOUR save (one save, one undo)
│   │        (manual + focus-out saves; not auto-save mid-typing; only functions without syntax errors)
│   ├── E3. documented node found again (ID comment / path / hash / git rename / body hash) ──► silent
│   │        └── ID comments deleted by hand ──► one notice: Restore / Keep removed
│   ├── E4. checkout / reset / rebase (HEAD jumped sideways) ──► gone nodes = "not on this branch", NO prompt
│   │        └── opening that list: on no local branch tip at all ──► promoted to Missing
│   └── E5. gone after an edit, or after a pull (HEAD moved forward) ──► "Missing" + one grouped notification
│         ├── [Archive]  ──► soft-deleted in place, waits ──► [Relink to…] ──► active again (or expires after N days)
│         ├── [Find it…] ──► pick the node it became
│         └── [Remove]   ──► delete it and its docs (confirmed; anything git doesn't have goes to the trash; V-NOC never empties it)
│
├── F. CHANGES SETTINGS ANY TIME  ("for the team or just me?" → vnoc.toml or local.toml)
│   ├── F1. IDs → none       IDs already in code stay; no new ones
│   ├── F2. IDs → all / on attach   preview first, then inject (per-node choices always win)
│   ├── F3. scope · call-tree depth · auto/manual sync
│   ├── F4. Rebuild index    delete the cache, re-index; docs untouched
│   └── F5. team ↔ solo      solo → team: docs stay private until you "Share all docs…" (with a preview)
│
└── G. SHARES WITH GIT
    ├── team: committed = vnoc.toml + nodes/ ;  ignored = local.toml + local/ (private docs, cache)
    ├── solo: only the local side exists; .vnoc/.gitignore is "*"  → nothing from .vnoc/ ever shows in git status
    ├── V-NOC writes .vnoc/.gitignore itself; your root .gitignore is never touched
    ├── two people document one function at once ──► same derived ID ──► same folder + same ID line ──► clean git merge
    └── a teammate clones ──► cache rebuilt ──► shared docs re-attach (ID comment or node.toml)
```

Not in v1, on purpose: **history/versioning, agent, tasks, logs** (and groups, pending review). Hooks: [`03-next/02-phase-plan.md`](03-next/02-phase-plan.md).

---

## Reading order

```
plan_vs/
│
├── README.md ........................................ you are here (the user tree above)
│
├── 00-requirements/ ................................. WHAT the user gets
│   ├── 01-vision-and-scope.md ....................... v1 in / out, personas, success criteria
│   ├── 02-user-journey.md ........................... the tree above, branch by branch
│   ├── 03-functional-requirements.md ................ FR-xx list with priorities
│   ├── 04-non-functional-requirements.md ............ speed, safety, git, platforms
│   └── 05-user-controls-and-config.md ............... who controls what; vnoc.toml + local.toml drafts
│
├── 01-decisions/ .................................... WHY
│   ├── 01-decision-log.md ........................... ✅ VS-D1…D57 · 🟡 VS-P1…P34 · 🔓 VS-O1…O10
│   └── 02-relation-to-old-plan.md ................... plan/ file by file: reuse · change · drop
│
├── 02-technology/ ................................... WITH WHAT (decision level)
│   ├── 01-architecture-at-a-glance.md ............... the boxes, arrows and main flows
│   ├── 02-running-rust-sidecar.md ................... how VS Code starts and talks to the Rust core
│   ├── 03-frontend-webview.md ....................... React + xyflow, the message bridge, the focus stack
│   ├── 04-storage-and-git.md ........................ where data lives, .vnoc/ layout, node.toml, two-IDs merge, cache, collisions
│   ├── 05-identity-and-archive.md ................... two keys, ID comments, modes + [[path]] rules, ladder, states, archive, save-time edits
│   ├── 06-distribution-and-integrations.md .......... VSIX per platform, stores, Cursor/opencode/pi/Claude Code later
│   └── 07-repo-and-drivers.md ....................... the monorepo; drivers as downloadable, signed executables
│
└── 03-next/ ......................................... WHAT HAPPENS NEXT
    ├── 01-review-checklist.md ....................... ★ what's still to answer before phase 2
    └── 02-phase-plan.md ............................. spikes, phase-2 docs, MVP milestones
```

Rule for this folder: **at most one sub-folder deep.** One topic per file.

---

## Start here

→ [`00-requirements/01-vision-and-scope.md`](00-requirements/01-vision-and-scope.md)
