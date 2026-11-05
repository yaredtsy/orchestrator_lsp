# 02.05: Identity: permanent IDs, ID modes, matching, node states, and how IDs get written

This is the part users will judge us on: **"I wrote a note, I refactored, is my note still there?"**
Ideas carried over from [`plan/04-drivers-and-ids/04-id-strategy.md`](../../plan/04-drivers-and-ids/04-id-strategy.md) and
[`plan/01-data-model/02-identity.md`](../../plan/01-data-model/02-identity.md): optional IDs, the docstring `ID:` line, the duplicate-ID policy.
Round 6 (2026-11-05): IDs are **derived by hash** at creation, and **the driver owns where they go** (Python: the docstring, as pylspt already does).

---

## 1. Two keys per node (VS-D41, VS-D50)

| Key | Looks like | Stored in | Changes? | Used for |
|---|---|---|---|---|
| **Location key** | `src/orders.py::OrderService.parse_order` | SQLite only | yes, on every rename/move | structure, call-tree edges, everything recomputable |
| **Permanent ID** | `0Q8V5W4T2F7KX9M3B6D1RZ4H7K` (26 chars) | the language's in-code ID (Python: an `ID:` line in a def/class docstring, a `# FileID:` comment at the top of a file) **and/or** the `.vnoc` folder name | **never** | docs, playgrounds, links: everything a person created |

- **A permanent ID is created only when something is attached** (doc, playground, a saved choice), or when injecting under mode `all`.
- **It's saved permanently the moment it's created**: as a folder with `node.toml`, in the code, or both. **Never generate IDs only in SQLite**: a cache-only ID would be lost on rebuild and point nowhere.
- Most nodes never get a permanent ID. They're just their location key in the cache.
- SQLite records **`id_source = code | anchor`**: was this node found by its in-code ID or by `node.toml`'s path/qualname? The UI shows a **solid link icon** (code) or an
  **outlined** one (anchor), so you can see which docs are attached more weakly.

### How an ID is made: derived by hash at creation (VS-D50)

```
id = base32( blake3( repo_relative_path + "::" + qualname + "#" + occurrence )[..16] )      → 26 characters

  repo_relative_path   forward slashes, relative to the git root:  src/orders.py
  qualname             OrderService.parse_order
  occurrence           1, or 2, 3… for a second def of the same name in the same file (default 1)

  files and folders:   qualname is empty  →  "src/orders.py::#1"   (a file and a folder can't share a path)
```

- **Why a hash (round-5 reply to R1):** the same function on two branches gets **the same ID**, so it gets the same `ID:` line and the same `.vnoc` folder. Git merges identical changes
  cleanly. With random IDs both would be different, which means a merge conflict in the source file.
- **Taken already? (VS-D71)** Before creating an ID: (1) ask the SQLite registry (a free early catch); (2) check the disk for `nodes/<shard>/<ID>/` and
  `local/nodes/<shard>/<ID>/` (authoritative). If either finds it, step the counter (`path::qualname#1#2`, then `#3`, …) and hash again until it's free.
  **Source files are not searched.** Only a folder collision can mix user data, and check (2) always catches it. This covers a function deleted and later
  re-created under the same name: its old, archived docs keep the old ID.
  - A **code-only duplicate** (the same `ID:` in two places, no folder involved) has no data attached and is repaired on the next sync: a copy matching
    `node.toml`'s anchor keeps the ID; otherwise the copy whose current `path::qualname` hashes to the ID keeps it; the other copy gets a new ID.
  - A **hash match on an archived folder** gives a new ID plus the prompt "An archived node used to be here: **Relink its docs?**"
- **Derived once, then permanent.** It doesn't follow later renames; that's the point of a permanent ID.
- **26 characters**, so sharding by the last 2 characters and everything else stays as it was.
  **Encode like a ULID** (Crockford base32 of the 128-bit number, with the 2 leftover bits in the *first* character). Plain RFC 4648 base32 puts them in the *last*
  character, which then has only 8 possible values, so there'd be 256 shards instead of 1,024 (R13, **decided: VS-D60**).
- **Two IDs on one node can still happen** (attach on branch A, rename on branch B, attach again on B → different inputs → different IDs). The re-key rules in 02-04 §6
  stay as the fallback. Their tie-break is now **the smaller ID wins** (there's no timestamp in a hash).
- **Old V-NOC IDs** (UUID v4 strings, 36 chars) are still read and kept. Sharding by their last 2 hex characters works the same.

> **🦀 Rust note: deriving the ID.**
> ```rust
> fn derive_id(path: &RepoPath, qualname: &str, occurrence: u32, salt: u32) -> PermId {
>     let mut h = blake3::Hasher::new();
>     h.update(path.as_str().as_bytes());                     // RepoPath guarantees forward slashes
>     h.update(b"::"); h.update(qualname.as_bytes());
>     h.update(b"#");  h.update(occurrence.to_string().as_bytes());
>     if salt > 0 { h.update(b"#"); h.update(salt.to_string().as_bytes()); }   // the "already taken" counter
>     let bytes: [u8; 16] = h.finalize().as_bytes()[..16].try_into().unwrap();
>     PermId(ulid::Ulid::from_bytes(bytes).to_string())       // the ulid crate's encoder = Crockford, spare bits up front
> }
> ```
> Reusing `ulid`'s encoder gets the right bit layout without writing base32 by hand. The ID isn't a time-based ULID; it's only *printed* like one.

## 2. IDs in code: the driver owns syntax and placement (VS-D54, VS-D49)

**Where and how an ID is written in source is the language driver's job, not the core's.** The core decides *whether* a node should have an in-code ID (§3) and *which* ID.
The driver decides *where it goes* and returns the text edit. It reports back, for every ID it reads, the **exact span and text** of that ID in the file.

**Python (pylspt):** an `ID: <id>` line **in the def's or class's docstring**. This already exists in pylspt:

- `src/edit.rs`: `doc_slot` / `plan_ids` work out the slot. If there's a docstring, the ID is added before its closing quotes. If there isn't, a new `"""ID: …"""` statement goes in.
  `plan_ids` takes the ID from a closure (`new_id`), so the core passes in the hash-derived ID instead of today's `Uuid::new_v4()`.
- `src/scan.rs` / `src/sem.rs`: `id_from_body` / `docstring_id` read it back.
- A body that doesn't parse, or an unclosed docstring, is never touched (`doc_slot` returns `None`). That's the parse guard of VS-D31, already in the driver.

```python
class OrderService:
    @retry(3)
    def parse_order(self, raw):
        """Parse the raw payload.

        ID: 0Q8V5W4T2F7KX9M3B6D1RZ4H7K
        """
```

- Folders never get IDs in source. V-NOC **never creates or edits `__init__.py`**, because that changes Python imports (namespace packages, pytest discovery). A folder's identity
  comes from its children's content hashes (§4, step 2). Supersedes FolderID and VS-P17.
- **File IDs are a comment at the top of the file (VS-D59).** Placement is still the driver's job. For Python:

  ```python
  #!/usr/bin/env python3
  # -*- coding: utf-8 -*-
  # Copyright 2026 Acme. Licensed under MIT.     ← a leading comment block (license header) stays first: license checkers expect it there
  # FileID: 0Q8V5W4T2F7KX9M3B6D1RZ4H7K          ← after shebang, encoding and any leading comment block; before the first statement
  """Orders module."""                           ← a comment before the module docstring doesn't change __doc__
  ```

  This is **new driver work**: pylspt writes def/class IDs today, not file IDs. Reading is tolerant (`#FileID:`, extra spaces); removal is the same exact-span rule as below.
  Non-code files (README, YAML…) still use `node.toml` only.
- **Trade-off to know about (R14):** a docstring ID is part of `__doc__`, so it shows in `help()`, IDEs' hover text and Sphinx/pdoc output. The old V-NOC worked the same way.

**`id_in_code` in `node.toml`: one per node, three values.** It replaces the earlier separate "don't re-add" flag.

| `id_in_code` | ID in code? | What sync does |
|---|---|---|
| `yes` | found | match by it; refresh `path` / `qualname` in `node.toml` |
| `yes` | **missing** | match by the ladder (§4). If found, the ID was **lost** (deleted by hand, a docstring rewrite, an AI agent) → **one grouped notice**: "3 V-NOC IDs were removed from code. **[Restore]** / **[Keep removed]**". Keep removed sets `never`. |
| `no` | — | match by the ladder. If the mode later becomes `all`, inject and set `yes`. |
| `never` | — | match by the ladder; **never inject**, whatever the mode or rule says. |

**Duplicate in-code IDs** (copy-paste, cherry-pick): keep the ID on the copy that matches the cached node (on a fresh clone, the one that matches `node.toml`'s `path` + `qualname`).
The other copy gets a new ID. If neither matches, ask (R9).

**Removing an ID: read tolerantly, remove exactly (VS-D43, VS-D49).**

1. When the driver **reads** IDs, it records each one's **exact span and text**. Reading is tolerant (`ID: x`, `ID:x`, extra spaces).
2. **Remove** deletes that recorded span **only if the file's current text at that span still matches exactly**. If V-NOC created the whole docstring (`"""ID: x"""`), the
   recorded span is the whole statement, so the docstring goes too. A docstring the user wrote loses only the `ID:` line.
3. **If the text no longer matches** (someone edited near it since the last read), nothing is changed: V-NOC tells the user and **shows the line** ("Open the line to remove it by hand").
4. Never a pattern search. Removal then sets `id_in_code = "never"`, so a path rule doesn't re-add it.

## 3. ID modes and path rules (VS-D42)

Modes: **`on-attach | all | none`**. Set as a project default, by path rules, or per node.

```toml
# .vnoc/vnoc.toml
[ids]
default = "on-attach"

[[path]]
path = "src/billing"
ids  = "all"

[[path]]
path = "src/billing/legacy"
ids  = "none"
```

**Resolution:** the node's own `id_in_code` > the **most specific** `[[path]]` rule (longest matching path, not file order) > `[ids] default`.

```
should_have_id_in_code(node) =
    if !node.in_scope            { return false }          // VS-D21: pulled-in and third-party nodes never get in-code IDs
    if !driver.can_carry(node)   { return false }          // the driver says. Python v1: files, classes, functions. Not folders, not non-code files.
    match node.id_in_code {                                // only exists if the node has a node.toml
        Some(Yes)   => true,
        Some(Never) => false,
        _ => match most_specific_rule(node.path).unwrap_or(ids.default) {
            All      => true,
            OnAttach => node.has_attachments(),
            None     => false,
        },
    }
```

- **Rules only add IDs, never remove.** Switching a folder to `none` stops new IDs. Removing is a separate explicit action: "Remove IDs in this folder" (exact span per ID, §2, with a preview).
- **A function that moves across rules keeps its ID.**
- **Rules follow folder renames:** when sync confirms a folder rename, it updates the rule's `path` in `vnoc.toml` (a `WorkspaceEdit` through `toml_edit`, 00-05).
- **A rule outside the index scope does nothing.** The effective-config view shows a warning on it.
- **Switching to `all` on a team repo:** the preview warns about the size of the diff and suggests committing it separately.
- **UI:** right-click a folder ▸ **"IDs for this folder and below"** ▸ All / On attach / None / Inherit. Folders with a rule get a badge in the tree.

| Change | Source edits |
|---|---|
| `none` → `on-attach` | add IDs to in-scope nodes that already have attachments (preview if > 1 file) |
| → `all` | add to every in-scope node that can carry one (preview + "commit separately" hint) |
| → `none` | **nothing**. Existing IDs stay and keep working (VS-D4). |
| **Add permanent ID** (one node) | one `ID:` line from the driver; creates the node folder + `node.toml` with `id_in_code = "yes"`. On a pulled-in node: "Add this file to the scope first?" Not offered on third-party nodes. |
| **Remove permanent ID** (one node) | the exact-span removal (§2); `id_in_code = "never"`; docs stay attached by `node.toml` |
| Solo mode chosen | mode defaults to `none` (VS-D25) |

## 4. Finding a node again: the matching ladder (VS-D44)

Runs during sync for every permanent ID **not found in code**. That covers `no` / `never`, a lost in-code ID, and non-code files.

| Step | Signal | Covers |
|---|---|---|
| 1 | Same **path and name** | nothing moved |
| 2 | Same **file content hash** at a new path | file or **folder renames** (a folder matches by its children's hashes) |
| 3 | **git rename detection** (`gix`) between the before and after trees | file moved **and** edited |
| 4 | **Body hash** with whitespace and comments stripped | function renamed or moved to another file |
| 5 | **Similarity score** | a clear winner matches; a close call **asks the user** (threshold: VS-O10) |
| 6 | No match | **Missing** (or "not on this branch", §5) |

- VS Code's `onDidRenameFiles` events tell steps 2–3 the answer up front, without searching.
- **The "before" state** (last known paths, hashes, fingerprints) comes from SQLite. **On a fresh clone** there's no cache, so it comes from git: the version of the file at
  **the last commit that changed that node's `node.toml`** (that's when `path` was last confirmed). Shallow clones may not have that commit, in which case only steps 1 and 6 run **[check] C19**.

> **🦀 Rust note: modelling the ladder.** Every step has the same shape, so a slice of function pointers is enough:
> ```rust
> enum MatchOutcome { One(NodeKey), Close(Vec<(NodeKey, f32)>), None }
> type Step = fn(&Lost, &Before, &Candidates) -> MatchOutcome;
> const LADDER: &[Step] = &[same_path_name, same_content_hash, git_rename, body_hash, similarity];
> ```
> Stop at the first `One(..)`. A `Close(..)` becomes the "Find it…" list. Each step is a pure function, so each gets its own table-driven test.

## 5. States, git events, archive (VS-D34, VS-D45)

**The problem.** A documented node disappears. Was it deleted, so the user should decide what happens to the docs? Or is it just on another branch and will come back, so the
user shouldn't be bothered? Timing can't tell, because `git checkout` and `git pull` both move HEAD. So we **test ancestry**:

```
on each sync:
    old = cache.last_synced_head        new = current HEAD          (Rust reads both with gix, VS-P28)

    if old == new                                  → a WORKING-TREE change                    → MISSING
    else if `old` is not in the repo any more      → can't tell (shallow clone, gc'd)          → NOT ON THIS BRANCH (the tip check catches real deletions)
    else if is_ancestor(old, new)                  → history moved FORWARD (pull, ff, commit)  → MISSING
    else                                           → history JUMPED SIDEWAYS (checkout, reset, rebase) → NOT ON THIS BRANCH

    cache.last_synced_head = new   (same transaction)
```

- **Rebase caveat:** when the user opens the "Not on this branch" list, a node found on **no local branch tip** is **promoted to Missing**.
- **Events:** `vscode.git` only *nudges* on a HEAD change. File events are **debounced (~200 ms) and processed as one batch**.
- **Startup scan:** compare `mtime` and `size` with the cache, and hash only the files that differ.
- **No git repo:** every disappearance is Missing.

```
                       found again (any ladder step)
        ┌────────────────────────────────────────────────────────────────────┐
        ▼                                                                    │
     ACTIVE ── gone, HEAD jumped sideways ──► NOT ON THIS BRANCH (computed) ─┤  quiet list, no prompt
        │                                         │ list opened & on no local branch tip
        │                                         ▼                          │
        └──── gone, same HEAD / HEAD forward ──► MISSING (computed) ─────────┘  badge + grouped notification
                                                  │      │      │
                                       [Archive]  │      │      │ [Remove] (confirm; V-NOC trash (VS-D70), 02-04 §7)
                                       ┌──────────┘      │      └─────────► REMOVED
                                       ▼                 │ [Find it…]
                                   ARCHIVED (in node.toml) ──┘
                                       ├── [Relink to…] ──► ACTIVE
                                       └── N days later ──► expiry (below)
```

**Archive is a soft delete in place:** `status = "archived"` and `archived_at` in `node.toml`. The folder doesn't move. It's shown in the **Archived** list with **Relink**, **Remove**, or expiry after **90 days** (configurable, VS-D56).

| Expired archived node | What happens |
|---|---|
| only **private** docs | **auto-expires to the V-NOC trash (VS-D70)** (never a plain delete, VS-D52) |
| any **shared** doc | **prompt** ("2 archived nodes expired. Remove?"), because removing them changes the repo |

- Missing and not-on-this-branch are **computed per machine, never stored**. Both clear themselves when the code comes back.
- **A teammate's deletion arrives by pull**: forward HEAD → Missing, so you're asked, unless their archive/remove came in the same pull.
- Archived nodes skip ladder steps 4–5. An in-code ID that reappears still brings one back.
- **Relink** rewrites `path` / `qualname`. If the target already has its own permanent ID, that's two IDs on one node: **merge** (02-04 §6) or cancel.
- **Never auto-delete orphans.**

## 6. The prompt (FR-71): only for Missing

```
┌───────────────────────────────────────────────────────────────┐
│ V-NOC: 2 documented nodes can't be found.        [Review] [×] │
└───────────────────────────────────────────────────────────────┘
Review → QuickPick:
   ⚠ OrderService.parse_order   src/orders.py · 2 docs (1 🔒)   → [Find it…] [Archive] [Remove]
   ⚠ helpers.py                  src/helpers.py · 1 doc          → [Find it…] [Archive] [Remove]
```

One per sync, non-blocking. Dismissing is safe: the nodes stay Missing with a badge. The "IDs removed from code" notice (§2) is a separate grouped notice.

## 7. Writing IDs into source (VS-D16, VS-D31, VS-P24)

**Rust never writes a source file.** If it did while the file had unsaved changes, VS Code would show "file changed on disk" and the user's undo history would break.
Rust (via the driver) **computes text edits** and returns them; the extension applies them.

| When | How it's applied |
|---|---|
| A **user action** (Add/Remove ID, the first doc in `on-attach` mode, Restore) | `workspace.applyEdit(WorkspaceEdit)`. File not dirty → save it. Dirty → the edit stays in the buffer and goes out with the user's next save. Undo works either way. |
| A **mode / rule change, or a bulk command** | One `WorkspaceEdit` across all files (one undoable refactor), then save the files that weren't dirty before. |
| **New functions** under `all` | **`workspace.onWillSaveTextDocument` + `event.waitUntil(edits)`**: the IDs become **part of the user's own save**. |

**Which saves inject (VS-D31):**

| `TextDocumentSaveReason` | When it fires | Inject? |
|---|---|---|
| `Manual` | ⌘S / Ctrl+S | ✅ |
| `FocusOut` | auto-save on focus change: the user **left** the editor | ✅ (otherwise people who only use auto-save never get IDs) |
| `AfterDelay` | auto-save after a pause **in typing** | ❌ skip; the next Manual/FocusOut save catches up |

**Parse guard:** inject only into functions that **parse without errors** (the driver parses with error recovery). **Scope guard:** only inside the index scope.

```
user saves (Manual or FocusOut)
  └─ onWillSaveTextDocument(e)    (file in scope, a rule says `all`, workspace trusted)
       e.waitUntil(
         client.sendRequest("vnoc/ids.forSave", { uri, text: e.document.getText(), version, reason })
       )                          → core → driver parses THIS text only
                                  → TextEdit[] for clean defs without IDs (skipping id_in_code = "never")
  └─ VS Code applies the edits to the buffer, then writes the file once
  └─ the new IDs are now stored permanently: in the docstrings. No node.toml yet; that appears when something is attached.
  └─ onDidSave → vnoc/filesChanged → content hash == the predicted one → no re-parse loop
```

- **The time budget (C8, answered from the API docs):** all extensions' `onWillSaveTextDocument` listeners share **about 1.5 s**. VS Code may skip the event in exceptional
  cases (shutting down with dirty files), ignores the edits if the document changed meanwhile, and stops calling a listener that keeps timing out or failing.
  So **the hook stays lightweight (VS-D66):**
  - On `onDidChangeTextDocument` (debounced), the core asks the driver for the planned ID edits **for that document version**, and keeps them.
  - At save time, the hook returns the stored edits if `document.version` still matches. Hash-derived IDs (VS-D50) make the plan deterministic, so precomputing is safe.
  - Version doesn't match (typed in the last few ms) → it computes for this one buffer if that's quick, else returns **no edits**. The next save catches up; nothing breaks.
- **API saves count as `Manual`.** `document.save()` from the extension, e.g. ⌘S inside the canvas (VS-D62), fires the hook with `TextDocumentSaveReason.Manual`, so save-time IDs work there too.
- Untrusted workspace → the participant isn't registered (VS-D18).

## 8. Scope, pulled-in files, third-party code, manual sync (VS-D20, VS-D21, VS-D22, VS-D32)

**Scope decides where call trees start, not where they can go.** And **"not indexed" never means "not resolved"**: ty still reads site-packages to resolve types.

| Code lives… | Counts as | In the call tree | In the index | IDs |
|---|---|---|---|---|
| in the scope | in-scope first-party | normal nodes, expandable | ✅ structure, up front | per mode/rules |
| in the repo, outside the scope (**including vendored code**) | **pulled-in** first-party | normal nodes, expandable, **dimmed** | its structure, **when a call tree first reaches it** | never in code; `node.toml` if something is attached |
| **outside the repo root**, or under **`site-packages`, `.venv`, `node_modules`** | **third-party** | a **leaf marked external** | ❌ | none |

- Third-party is defined **by location**, so vendored code inside the repo is first-party.
- **Jump to code** on an external leaf opens the library file **read-only** **[check] C15**.
- Docs can be attached to pulled-in nodes (`node.toml` only). Not to third-party leaves in v1.

**Known v1 limit (written down on purpose):** a path that goes **through a library and back into your code** stops at the library leaf:

```
your_view()  ◄── called by Django's request handler     → the tree for Django's handler isn't built; your view's own tree is
map(my_callback, items)                                 → `map` is an external leaf; `my_callback` doesn't appear under it
```

**Auto vs manual sync:**

| | Auto (default) | Manual |
|---|---|---|
| File events | debounced, one batch | only **counted**: `V-NOC: 12 changed · Sync` in the status bar |
| Call trees | fresh | **still correct**: cached expansions are checked against their dependency hashes when read |
| Falls behind until Sync | nothing | tree structure, new nodes, Missing detection, ID injection **[check] C10** |
| HEAD changes | ancestry test at the next sync | the same |

Next: [`06-distribution-and-integrations.md`](06-distribution-and-integrations.md)
