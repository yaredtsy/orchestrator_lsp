# 02.04: Storage and git: where data lives, the two sides of `.vnoc/`, node folders, collisions

Decisions: VS-D36 (three places), VS-D37 (shared vs local, solo = local only), VS-D38 (node folders), VS-D39 (one `node.toml`),
VS-D40 (two IDs on one node), VS-D35 + VS-D52 + VS-D70 (Remove and the V-NOC trash), VS-D46 + VS-D53 (the cache), VS-D50 (hash-derived IDs), VS-D51 (repo files are untrusted),
VS-D55 (one `.vnoc/` per git root), VS-D58 (moving between sides), VS-P24 (who writes what).
Round 5 (2026-11-05) replaced this file's earlier `anchor.toml` / `state.toml` / `<slug>--<id>` design; round 6 brought back hash-derived IDs. The old rows are marked ⤴ in the decision log.

## 1. Where data lives (VS-D36)

| Place | Holds | Can be rebuilt? |
|---|---|---|
| **Source code** (in git) | the functions, plus in-code IDs when enabled (Python: an `ID:` line in the docstring, placed by the driver) | it *is* the truth |
| **`.vnoc/` files** | only what a **person created** and V-NOC **can't recompute**: docs, playgrounds, `node.toml`, config | ❌ (this is why they're files, not DB rows) |
| **SQLite cache** | everything recomputable | ✅ delete any time; rebuilt from the two above |

> **The test for "does it go in a file?"** Could V-NOC compute it again from the code plus the other files? Then it goes in the cache, not in `.vnoc/`.

## 2. The `.vnoc/` layout

```
<git root>/.vnoc/
├── vnoc.toml                    shared   the team's config (stays inside .vnoc/, not at the repo root)
├── nodes/                       shared   node folders, sharded (§4)
│   └── 7K/
│       └── 0Q8V5W4T2F7KX9M3B6D1RZ4H7K/
│           ├── node.toml                 the ONE node.toml for this node (§5)
│           ├── why-we-parse-twice.md
│           ├── play-basic.py
│           └── assets/diagram.png
├── local.toml                   local    my overrides (ignored)
├── local/                       local
│   ├── nodes/7K/0Q8V…7K/                 private node folders, same sharding
│   │   └── my-todo.md                    (no node.toml here: this node's one node.toml is on the shared side)
│   ├── cache/                            ── MACHINE DATA ──  index.sqlite (+ -wal, -shm), lock
│   └── trash/<timestamp>_<node-id>/      written by Remove, with a trashed.toml (VS-D70)
└── .gitignore                   written by V-NOC at setup: team `local/` + `local.toml`; solo `*` (§3)
```

**Where `.vnoc/` goes:**

- **One per project, at the git root.** No git: at the workspace folder root.
- **One per git root, not per workspace root (VS-D55).** Multi-root workspace roots inside the same git repo share that repo's one `.vnoc/`. Roots in different repos (or with no git) get one each.
- **"Set up here" on a sub-folder** adds that folder to the **scope**. It never creates a second `.vnoc/`.
- **Opened folder is a sub-folder of the repo** (`code repo/services/api`): `.vnoc/` sits *above* the opened folder. The extension watches it with a
  `RelativePattern` based on the git root **[check] C18**. It won't appear in VS Code's Explorer, which is fine: V-NOC's own views show its contents.

**Other rules:**

- **V-NOC writes `.vnoc/.gitignore` itself** and never touches the repo's root `.gitignore`.
- **Quick look has no `.vnoc/` at all.** Its cache is in VS Code's `context.storageUri` (VS-D17). "Make it a project" creates `.vnoc/`.

> **🧩 VS Code note: watching outside the opened folder.**
> `workspace.createFileSystemWatcher("**/*")` only covers the opened folders. For a folder above them you pass a `RelativePattern(baseUri, "**/*")` with
> `baseUri` = the git root. VS Code then watches that folder too, even though it isn't open in the Explorer.

## 3. Shared vs local (VS-D37)

Both sides use **the same layout**. Rust reads both and merges them:

| What | Merge rule |
|---|---|
| Settings | `local.toml` **overrides** `vnoc.toml`, key by key. The "effective config" view (00-05) shows where each value came from. |
| Docs, playgrounds | **Both shown together.** Private ones carry a 🔒 marker. |
| `node.toml` | **Exactly one per node**, on the side picked by §5. |

- **The test for "shared":** if a teammate's V-NOC would act differently without this fact, it's shared.
  Example: `id_in_code = "never"` under a shared ID mode is shared. Without it, the teammate's sync would put the ID back.
- **Placement is enforced in the Rust write path, not in the UI** (the 🦀 note below).
- **One node can have data on both sides.** Shared docs in `nodes/7K/<id>/`, my private notes in `local/nodes/7K/<id>/`: same ID, same folder name, two parents.
  The canvas shows them together (private ones 🔒). Only one of the two folders holds the node's `node.toml` (§5).

### Moving between sides = moving the folder, never editing `.gitignore` (VS-D58, VS-D63)

**`.vnoc/.gitignore` is written at setup, from the mode:**

```gitignore
# team mode                 # solo mode
local/                      *          ← ignores all of .vnoc/, the .gitignore itself included,
local.toml                             so nothing from .vnoc/ shows in git status
```

**Sharing and un-sharing never touch it.** Whether something is committed depends only on **which parent folder it's in**: `nodes/` is committed, `local/nodes/` isn't.
So sharing or un-sharing is a **move**. Only an explicit **mode switch** rewrites `.gitignore` (below).

| Action | Moves | With |
|---|---|---|
| **Share this doc** | one file `local/nodes/7K/<id>/x.md` → `nodes/7K/<id>/x.md` | `WorkspaceEdit.renameFile` (the doc may be open, VS-P24) |
| **Share this node** | the whole folder `local/nodes/7K/<id>/` → `nodes/7K/<id>/` | if the target folder exists, the files are **merged into it**; a same-name clash → `notes (2).md` + `vscode.diff` |
| **Make private** (doc or node) | the other way | same |
| `node.toml` | follows the §5 rule after each move | automatic |

**Team mode and solo mode** decide where new things go by default, the ID default, and the `.gitignore` written at setup:

| | Team | **Solo ("just me")** |
|---|---|---|
| New docs go to | `nodes/` (shared) | `local/nodes/` (private). V-NOC creates **only the local side**: `local.toml`, `local/` |
| Config | `vnoc.toml` (+ my `local.toml`) | `local.toml` only |
| Default ID mode | `on-attach` | **`none`** (in-code IDs would be committed with the source and seen by teammates) |
| `.vnoc/.gitignore` | `local/` + `local.toml` | `*` (VS-D63) |
| In `git status` | `vnoc.toml`, `nodes/`, `.vnoc/.gitignore` | **nothing** from `.vnoc/` |

**Switching modes:**

- **Solo → team:** V-NOC creates `vnoc.toml` (seeded from `local.toml`'s team-level keys, shown in a preview) and rewrites `.gitignore` from `*` to `local/` + `local.toml`.
  Everything you have is under `local/`, so it stays ignored: nothing becomes committable by itself. Your docs stay private.
  Then it offers **"Share all docs…"**: a preview of every folder that would move to `nodes/` (and so become committed), with checkboxes.
- **Team → solo:** `.gitignore` becomes `*` (files git already tracks stay tracked; `.gitignore` doesn't untrack them). New docs default to private. Existing shared folders **stay where they are**, because they belong to the team. Moving them to `local/` would
  delete them from the repo at your next commit, and teammates would lose them. "Make all private…" exists but warns exactly that, with a preview.

> **🦀 Rust note: making the rules unbreakable with types.**
> Give the write path no way to say "just write this file there":
> ```rust
> pub enum Side { Shared, Local }
>
> impl UserDataStore {
>     // The caller says WHAT changes. The store works out WHERE, and returns the file operations.
>     pub fn attach_doc(&self, node: &PermId, title: &str, side: Side) -> FileOps { /* + moves node.toml if §5 says so */ }
>     pub fn set_id_in_code(&self, node: &PermId, v: IdInCode) -> FileOps { /* §5 decides the side */ }
>     pub fn archive(&self, node: &PermId) -> FileOps { /* status + archived_at in the one node.toml */ }
> }
> ```
> There's no `pub fn write(path, bytes)`. If the only public functions are the use cases, the compiler guarantees nobody bypasses the rules.
> `FileOps` is data. The extension applies the ones that touch user-visible files as a `WorkspaceEdit` (VS-P24).

## 4. Node folders (VS-D38)

- **One folder per node, named by its permanent ID** (26 characters, 02-05 §1), sharded by the **last 2 characters**: `nodes/7K/0Q8V5W4T2F7KX9M3B6D1RZ4H7K/`.
  - *Why shard:* git hosts and web UIs get slow on directories with thousands of entries.
  - *Why the last characters:* the ID is printed like a ULID, whose **first** character carries only 3 bits (values 0–7). The last two carry 5 bits each → 1,024 evenly filled shards.
  - *Why from day one:* changing the layout later moves every folder in one huge commit.
- **A folder exists only when something is attached** (a doc, a playground, a saved state). So the folder count equals the number of annotated nodes, never the code size, in every ID mode.
- **Contents:** `node.toml`, any number of `.md` docs, playgrounds, `assets/`. **The folder is the doc list**: no index file to keep in sync.
- **Each `.md` gets frontmatter**, so a PR diff says which function the doc is about:
  ```markdown
  ---
  node: 0Q8V5W4T2F7KX9M3B6D1RZ4H7K
  hint: src/orders.py::OrderService.parse_order
  ---
  # Why we parse twice
  ```
  `hint` is rewritten only when the user saves that doc, and is **never used for lookup**. If `node:` and the folder disagree (a file moved by hand), **the folder wins**, and `node:` is fixed on the next save.
- **Links between docs:** `vnoc://node/<id>`. The cache keeps them in a `doc_links` table for "what links here".

> **🦀 Rust note: why Crockford base32.** 26 characters, no I, L, O, U, **uppercase only**, which is safe on case-insensitive macOS/Windows file systems. Two IDs that differ
> only in case can't exist, so they can never collide as folder names. The `ulid` crate's encoder does it (02-05 §1 shows `derive_id`).

## 5. `node.toml`: one per node (VS-D39)

```toml
# .vnoc/nodes/7K/0Q8V5W4T2F7KX9M3B6D1RZ4H7K/node.toml
path        = "src/orders.py"
qualname    = "OrderService.parse_order"
id_in_code  = "yes"            # "yes" | "no" | "never"   (02-05 §2)
status      = "active"         # "active" | "archived"
# archived_at = 2026-11-05     # only when archived
```

**Which side holds it:**

```
node.toml goes on the SHARED side if
    the node has any shared doc / playground
 OR id_in_code = "never" while the effective ID mode comes from vnoc.toml (shared)
otherwise LOCAL.
```

- **All node state lives in that one file.** Archive is a fact about the **node**, so the team's archive decision also covers my private notes on the same node.
- The side changes when the rule's answer changes. Sharing the first doc moves `node.toml` to shared; making the last shared doc private moves it back. Both moves are `WorkspaceEdit.renameFile`.
- **`path` and `qualname` are kept current**, even when `id_in_code = "yes"`. That's what lets a fresh clone without the comment find the node.
  They're rewritten only when a sync **confirms a move** (working-tree change or forward HEAD), never for "not on this branch".
- Not stored, on purpose: Missing / not-on-this-branch (computed per machine), hashes and fingerprints (cache), "last seen" times (they'd change on every sync).

## 6. Two IDs that point at one node (VS-D40)

**Mostly it doesn't happen any more.** IDs are derived from `path::qualname#occurrence` (VS-D50), so two people attaching to the same function on two branches get **the same ID**, the
same folder and the same `ID:` line, and git merges that cleanly. **What's left** is when the inputs differed: one branch renamed or moved the function and someone attached on each side
(different paths → different IDs), or the counter kicked in on one machine only. Then there are two folders, **git sees no conflict**, and V-NOC finds it during sync.

| Match between the two | What happens |
|---|---|
| **Strong**: same in-code ID, or same `path` + `qualname` | **Fixed automatically**: the losing folder is renamed into the winner's (`WorkspaceEdit.renameFile`, docs may be open). **Shared beats local.** **Shared vs shared: the smaller ID wins** (plain string order; a hash has no timestamp), so every machine makes the **same** rename and their uncommitted changes are identical. A one-line info says what moved. |
| **Weak**: fingerprint or similarity | **Ask** the user ("These look like the same function. Merge?"). |

- **The user only has to choose on a real content collision:** the same file name in both folders on the same side (`notes.md` vs `notes.md`). V-NOC keeps both, renames
  one to `notes (2).md`, and shows them with `vscode.diff`.
- If the losing ID is also **in code**, its `ID:` line is replaced with the winner's, using the driver's recorded span and the exact-text check (VS-D49). This is a source edit, so it
  goes through `WorkspaceEdit` and only happens in a trusted workspace.
- `.gitattributes` merge drivers are **optional** only, since they need setup in every clone.

## 7. Remove, and the V-NOC trash (VS-D70)

**Remove** (from a Missing / Archived node) **moves** the node's files on both sides into **`.vnoc/local/trash/<timestamp>_<node-id>/`**, after a confirmation that
lists the files. Every Remove works this way: private or shared, committed or not, in team or solo mode. The OS trash is never used, and there's no "is it in git?" check.

- The relative paths are kept inside that folder, next to a **`trashed.toml`** (`node`, `side`, `file`, `removed_at`). It's under `local/`, so it's never committed.
- The move is a `WorkspaceEdit.renameFile`, so **an open, dirty file follows the move and keeps its unsaved text** (the editor now shows the trash path).
- A **"Recently removed"** list in the V-NOC UI offers **Restore**, which moves the files back. If the node no longer exists, it comes back as **Missing**.
- **Restore never overwrites.** If a path exists again, that's reported as a conflict and the removed copy stays in the trash.
- The file watcher sees each move as a **delete plus a create**.
- **Never cleared automatically.** The V-NOC UI shows the trash's size and an **"Empty V-NOC trash"** button: the only permanent delete, and it needs a click.

The facts above were measured in spike S3 on **macOS only**.

**V-NOC never permanently deletes user-written content without a click (VS-D52).** Expiry and Remove go to the V-NOC trash; only the user empties it. Never auto-delete orphans either
(a folder whose node can't be found is Missing, not garbage). Archive expiry: 02-05 §5.

## 8. The cache: SQLite (VS-D46)

- `rusqlite` with the **`bundled`** feature (SQLite compiled in, so there are no system library surprises). **WAL** mode.
- **One writer thread.** Other threads send it work and get results back over a channel.
- **`PRAGMA user_version`** = schema version. A mismatch or corruption means delete and rebuild (NFR-13).
- Behind a **`GraphStore` trait with domain methods only**. There's no raw-query method, and tests run against the trait. Turso is a possible later backend (same file format, pre-1.0 today).

**Tables (sketch):**

| Table | Holds |
|---|---|
| `files(path, hash, mtime, size)` | what the startup scan compares |
| `nodes(key, kind, name, parent_key, file, start_line, end_line, perm_id, id_source)` | structure. `key` = the location key `path::qualname`; `id_source` = `code` or `anchor` |
| `docs(perm_id, file, title)`, `doc_links` | the doc list and `vnoc://` links |
| `refs(caller_key, name, line, col, kind)`, `import_aliases` | call sites, before resolution |
| `expansions(fn_key, ctx_key, version)`, `edges(fn_key, ctx_key, ord, callee_key, callee_ctx_key, call_line)` | lazy call-tree layers |
| `expansion_deps(fn_key, ctx_key, version, file_path, file_hash)`, `callers` | what each expansion depends on |
| parsed `.vnoc` files, keyed by mtime | so startup doesn't re-read every `node.toml` |
| `key_moves(old_key, new_key, at_sync)` | location-key renames seen by sync, so saved canvas tabs can translate their keys when restored (VS-D64). Chains are followed (`a → b → c`); rows older than `cache_days` are dropped |
| `meta(last_synced_head, …)` | HEAD awareness (VS-D34) |

**Validation, not deletion:**

- **Cache entries are checked against their dependency file hashes when read**, not deleted when a file changes.
- **Several versions per key are kept** and evicted after **30 days** unused (configurable, VS-D56). Switching back to a branch finds its expansions still there. That's what makes branch switching cheap.
- **When the Python environment changes, clear only the call-resolution data** (`refs` resolution, `expansions`, `edges`, `callers`). Keep `files` and `nodes`, because structure
  doesn't depend on the environment (VS-D53). **What counts as an environment change** is narrow: the **lockfile** (`uv.lock`, `poetry.lock`, …), the **venv**, and the
  **dependency sections and `[tool.ty]`** of `pyproject.toml`. Editing `[tool.black]` or `[tool.pytest…]` clears nothing. The driver reports which inputs count, since this
  is language-specific (the TS driver will name `package-lock.json` and `tsconfig.json`).
- **Never holds user data.** Code text isn't stored either; it's read from the file or the open buffer.

> **🦀 Rust note: one writer thread + a channel.**
> ```rust
> enum Job { ApplySync(SyncPlan, oneshot::Sender<Result<()>>), /* … */ }
> let (tx, rx) = std::sync::mpsc::channel::<Job>();
> std::thread::spawn(move || {
>     let mut conn = Connection::open(path)?;                // only this thread ever writes
>     for job in rx { match job { Job::ApplySync(plan, reply) => { let _ = reply.send(apply(&mut conn, plan)); } } }
> });
> ```
> SQLite allows only one writer at a time anyway. Making it one *thread* means you never see `SQLITE_BUSY` in your own process. Readers open their own connections (WAL lets them read while the writer writes).

## 9. Collision cases

| # | Situation | What happens |
|---|---|---|
| 1 | The DB file | Under `local/`, never committed. |
| 2 | Alice documents `a()`, Bob documents `b()` | Different folders, so no conflict. |
| 3 | Both edit the same shared doc | Normal Markdown conflict. |
| 4 | Both attach to the same function on two branches | **Same derived ID** (VS-D50) → the same folder → their docs sit side by side. No conflict unless both wrote a doc with the same file name (then case 3). |
| 5 | …and both **inject the ID** into the docstring | The **same `ID:` line** at the same spot → git merges identical changes cleanly. |
| 5b | Attach on branch A, rename the function on branch B, attach again on B | Different inputs → two IDs → two folders. Sync sees a strong match → smaller ID wins, folders merged (§6). |
| 6 | A rename on one branch, a doc added on another | In-code ID finds it; else the ladder (02-05 §4); else Missing → Find it… |
| 7 | **Switching branches** (checkout, reset, rebase) | **Not Missing.** "Not on this branch", silently (VS-D34). |
| 7b | **Pulling a teammate's deletion** | **Missing** (forward HEAD), unless their archive/remove of the shared `node.toml` came in the same pull. |
| 8 | `vnoc.toml` edited on two branches | Normal TOML conflict. `local.toml` is never shared, so it never conflicts. |
| 9 | A teammate renamed a documented function and their V-NOC wasn't running | Every teammate's sync updates the same `node.toml` the same way → identical uncommitted changes that merge cleanly, but they show in `git status` after the pull (R5). |

Next: [`05-identity-and-archive.md`](05-identity-and-archive.md)
