# 09.06 Organizing a language's code: the repos, the crates, the folders

Written 2026-11-10 after reading `pylspt/` (commit `670ae79a15`, its `Cargo.toml`, `src/`, `plan/01-crates/04-public-vs-private-api.md`) and
`vnoc/drivers/python/`. The goal you gave: **grow fast, let contributors join easily, and keep adding, updating and removing languages cheap.**
Everything below is decided with that goal; where it changes an earlier rule, §10 says so.

```
organizing
├── 1. two kinds of code per language: the analyzer and the driver
├── 2. where each lives (the one rule, and when to break it)
├── 3. a new language: the folder inside vnoc
├── 4. pylspt: the target layout
├── 5. pylspt: what moves where
├── 6. the public API: how a library stays easy to depend on
├── 7. tests contributors can add in five minutes
├── 8. versions, releases, and changes that touch two repos
├── 9. the vendored ruff: updating it on purpose
├── 10. add, update, remove: the three everyday jobs
└── 11. the order to do it in
```

## 1. Two kinds of code per language

Every language has two layers of code, and they answer different questions. [`06-drivers.md`](../06-drivers.md) §7 draws the line; this file is
about where those two layers *live*.

| Layer | Knows | Does not know | Python today |
|---|---|---|---|
| **analyzer** | the language: parsing, names, where text may go, call resolution | V-NOC's wire, IDs, `.vnoc/` | `pylspt` (its own repo) |
| **driver** | V-NOC's wire: JSON-RPC, method names, capabilities, UTF-16 positions | the language's grammar | `vnoc/drivers/python` |

The test for any function: **would a tool that never heard of V-NOC want this?** Yes → analyzer. No → driver.

Keeping the two apart is what makes every later job cheap: a protocol change touches only drivers, a parser upgrade touches only an analyzer, and
an analyzer can leave the repo (or a new one can join it) without rewriting the driver.

## 2. Where each lives

**The rule: a driver always lives in `vnoc/drivers/<lang>/`. Its analyzer starts in the same folder, as its own crate, and moves to its own repo
only when it meets one of four conditions.**

Why the driver always stays in vnoc:

- The conformance test, the protocol crate and the packaging script are there. A protocol change and the fix to every driver is **one pull request**,
  checked by one CI run.
- A contributor clones one repo, runs one command, and has the whole product.

Why the analyzer starts there too: a new contributor's first driver is a few hundred lines on top of tree-sitter or the language's own parser
library. Two repos, a pin and a release step for that is all cost and no benefit.

**Move an analyzer to its own repo when at least one of these is true:**

| Condition | Why it forces a move | Python |
|---|---|---|
| it needs its **own toolchain** or a **vendored compiler** | it would make every vnoc build and CI run pay for it | ✅ Rust 1.98.0 pinned; 42 MB of vendored ruff / ty |
| it is **useful without V-NOC**, and someone outside wants it | its users should not need to clone an editor extension | ✅ a general Python analyzer |
| its **build or test time** slows vnoc's CI noticeably **[measure: more than ~2 minutes added]** | contributors to the UI should not wait for it | ✅ ty compiles for minutes |
| its **licence** differs from vnoc's | keeps the licence question per repo | — |

Once an analyzer has moved, it owes one thing: **it builds and passes its tests for all eight release targets in its own CI**, because vnoc
compiles it from source on every release runner ([`../11-git-and-release/04-what-this-changes.md`](../11-git-and-release/04-what-this-changes.md) §3).

So Python stays the one split-out analyzer, and every other language starts inside vnoc. Section 10 shows the move when a second one qualifies.

```
vnoc/                                         one clone = the whole product
└── drivers/
    ├── python/       driver only ──────────┐ depends on (git + tag)
    ├── go/           driver + analyzer      │
    └── typescript/   driver + analyzer      │
                                             ▼
pylspt/                                       its own repo, because it meets 3 of the 4 conditions
```

> **🦀 Rust note: why "its own crate" even inside one folder.** A crate is the unit the compiler checks imports for. If the analyzer is a module of
> the driver crate, nothing stops `use vnoc_driver_protocol::…` from creeping into it, and moving it out later means untangling those imports first.
> If it is a separate crate whose `Cargo.toml` does **not** list `vnoc-driver-protocol`, such an import does not compile. The boundary is then kept
> by the compiler, not by code review, and moving the analyzer out later is a `git mv` plus one changed line in a `Cargo.toml`.

## 3. A new language: the folder inside vnoc

The same shape for every language, so a contributor who has read one driver can read all of them.

```
vnoc/drivers/<lang>/
├── README.md                    what is supported (tier 1 / 2 / 3), how to build and test, known gaps
├── DECISIONS.md                 the decision sheet from 05-step-by-step.md §1, filled in
├── vnoc-driver-<lang>.json      the manifest: extensions, file names, language ids, protocol (S14)
├── Cargo.toml                   a small workspace: the two crates below
├── analysis/                    crate `<lang>-analysis`        ← knows the language; never imports the protocol crate
│   ├── Cargo.toml
│   ├── src/lib.rs               the public API: outline(text) → defs, plan_marker(text, def, id_text) → edit, …
│   └── tests/
│       ├── fixtures/*.<ext>     small source files
│       └── snapshots/           expected outlines (§7)
├── driver/                      crate `vnoc-driver-<lang>`     ← the binary; knows the wire
│   ├── Cargo.toml               depends on ../analysis and ../../../crates/vnoc-driver-protocol
│   └── src/
│       ├── main.rs              the JSON-RPC loop (copy the Python driver's; it is the reference)
│       ├── convert.rs           analyzer types → protocol types; byte offsets → UTF-16
│       └── methods.rs           one function per vnoc/* method
└── conformance/
    ├── input.<ext>              with an emoji in it (catches UTF-16 mistakes)
    └── expected-outline.json
```

Rules for this folder:

1. **The analyzer's public API takes text and returns plain data.** No file reads, no globals, no printing. It is then easy to test, easy to fuzz,
   and the driver can feed it an unsaved buffer.
2. **Only `driver/` may print**, and only protocol frames to stdout. Put `#![deny(clippy::print_stdout, clippy::print_stderr)]` at the top of
   *both* crates (the Python driver already does this in `main.rs`).
3. **Join the root vnoc workspace** (`members = ["crates/*", "drivers/<lang>/*"]`) unless the language needs its own toolchain. Then one
   `cargo test` at the root tests it, and it shares vnoc's `Cargo.lock`. A driver with its own toolchain keeps its own workspace, as Python does.
4. **A driver not written in Rust** (for example a TypeScript driver using the TypeScript compiler API) has the same two parts as two packages, and
   the same rule: the analysis package never imports the wire types.

## 4. pylspt: the target layout

Today pylspt is one crate at the repo root: `src/` holds 17 modules and five binaries, the outline and ID edits compile together with ty, salsa
and the call-tree solver, and the learning material sits next to the code. You chose **two crates** for it. The full target:

```
pylspt/                                  GitHub: rename rust_python_parser → pylspt (§8)
├── README.md                            what it is · the two crates · build · who uses it (V-NOC)
├── CONTRIBUTING.md                      setup · the test loop · how to add a fixture · how to update ruff
├── CHANGELOG.md                         one section per tag
├── LICENSE                              yours; vendor/ruff/LICENSE stays beside the vendored code
├── Cargo.toml                           a VIRTUAL workspace (no [package]): members = ["crates/*"],
│                                        and [workspace.dependencies] with the ruff, ty and salsa pins in ONE place
├── Cargo.lock
├── rust-toolchain.toml                  1.98.0 (vnoc uses the same version: ../11-git-and-release/04 §1)
├── .gitattributes                       LF everywhere; fixtures and snapshots never converted (../11-git-and-release/02 §6)
│
├── crates/
│   ├── pylspt-syntax/                   FAST. ruff_python_parser + ruff_python_ast + ruff_text_size. No salsa, no ty.
│   │   ├── src/
│   │   │   ├── lib.rs                   the public API and the re-exports (§6)
│   │   │   ├── outline.rs               defs, kinds, ranges, `<locals>` qualnames, per-def syntax_error
│   │   │   ├── ids.rs                   reading an `ID:` line from a docstring, as text
│   │   │   ├── edit.rs                  planning the insert / replace of an ID line (text + span, old coordinates)
│   │   │   ├── header.rs                where a header comment may go (after shebang, encoding, licence)
│   │   │   └── lines.rs · positions.rs  line index, byte ↔ line/column helpers
│   │   └── tests/                       fixtures + snapshots (§7)
│   │
│   ├── pylspt-analysis/                 SLOW. ty + salsa. Depends on pylspt-syntax.
│   │   ├── src/
│   │   │   ├── lib.rs                   the public API: project, call tree, expand, cancel
│   │   │   ├── db.rs · sem.rs · attrs.rs · mro.rs · stable.rs
│   │   │   ├── interp/                  the call-tree solver (as today)
│   │   │   └── legacy/                  nodes.rs, scan.rs, inject.rs: parity with the old web driver's JSON (§5)
│   │   └── tests/
│   │
│   └── pylspt-dev/                      the developer CLI. ONE binary with subcommands instead of five binaries:
│       └── src/main.rs                  `pylspt-dev ast | attrs | inject | mro | calls …`
│
├── vendor/ruff/                         the trimmed ty closure, unchanged except the documented patch
│   └── PATCHES.md                       NEW: upstream commit (ac201b8), every file edited, how to re-apply (§9)
│
├── docs/                                the learning material, moved here unchanged
│   ├── plan/
│   ├── tutorial/
│   └── experience/
│
├── scripts/
│   ├── update-ruff.sh                   §9
│   └── run-on-project.sh                was run-jarvis.sh; the project path comes from an argument, not a hard-coded home folder
│
├── out/                                 gitignored: what the dev CLI writes (cli-wire.json goes here, not at the root)
└── .github/workflows/ci.yml             §7
```

Three decisions in that tree, and why:

| Decision | Why it serves the goal |
|---|---|
| **Learning material under `docs/`** | The root shows only what builds. A contributor sees `crates/`, `vendor/`, `docs/` and knows where to look. Nothing in the notes changes; links between them stay relative and keep working. |
| **One dev binary with subcommands** | One thing to build, one `--help` that lists every tool. Adding a tool is one subcommand, not a new `[[bin]]` entry. |
| **A virtual workspace at the root** | Both crates share one lockfile and one place for the ruff / salsa pins (the pylspt plan's "one source, one copy" rule still holds). `cargo test -p pylspt-syntax` runs in seconds because it never builds ty. |

The crate names say what each one costs: `-syntax` is cheap, `-analysis` is the expensive one. The driver for 0.1 depends on `pylspt-syntax`
only; `pylspt-analysis` joins it in 0.2 with call trees.

> **🦀 Rust note: a virtual workspace, and inherited dependencies.** A `Cargo.toml` with `[workspace]` and no `[package]` is a *virtual manifest*:
> it builds nothing itself, it only groups crates. Declaring a dependency once at the root and inheriting it keeps every crate on the same version:
> ```toml
> # pylspt/Cargo.toml
> [workspace]
> members  = ["crates/*"]
> resolver = "3"
>
> [workspace.dependencies]
> ruff_python_parser = { path = "vendor/ruff/crates/ruff_python_parser" }
> ruff_python_ast    = { path = "vendor/ruff/crates/ruff_python_ast" }
> ruff_text_size     = { path = "vendor/ruff/crates/ruff_text_size" }
> ty_python_semantic = { path = "vendor/ruff/crates/ty_python_semantic" }
> salsa              = { version = "0.28.2", default-features = false, features = ["compact_str", "macros", "salsa_unstable", "inventory"] }
> pylspt-syntax      = { path = "crates/pylspt-syntax", version = "0.3.0" }
> ```
> ```toml
> # crates/pylspt-syntax/Cargo.toml
> [dependencies]
> ruff_python_parser = { workspace = true }      # "use the root's line for this one"
> ruff_python_ast    = { workspace = true }
> ruff_text_size     = { workspace = true }
> ```
> The vendored ruff crates keep their own `{ workspace = true }` lines against `vendor/ruff/Cargo.toml`, as today: a path dependency may point into
> another workspace. Nothing in `vendor/` changes for this.

## 5. pylspt: what moves where

Today's 17 modules, sorted. The `ruff_db` / `ty` column is what decides the crate: `pylspt-syntax` may not use either.

| Module (lines) | Uses `ruff_db` / ty today | Goes to | Note |
|---|---|---|---|
| `lines.rs` (33), `positions.rs` (91), `check_positions.rs` (61) | no | **syntax** | |
| `qname.rs` (82) | yes | **syntax** | the `<locals>` naming is syntax; read the AST from `ruff_python_parser` instead of `parsed_module` |
| `defs.rs` (192) | yes (`parsed_module`, `Db`) | **syntax** | the same change: parse text directly. It becomes `outline.rs`, the one walker (06-drivers §7, "the outline walk") |
| `edit.rs` (285) | yes; mints a UUID | **syntax** | split into `ids.rs` (read) and `edit.rs` (plan). The caller passes the ID text; the UUID default goes (06-drivers §7) |
| `stable.rs` (25) | no | **analysis** | stable ids for call-tree nodes |
| `db.rs` (29), `sem.rs` (607), `attrs.rs` (123), `mro.rs` (146) | yes | **analysis** | |
| `interp/` (18 files) | yes | **analysis** | unchanged |
| `nodes.rs` (30), `scan.rs` (199) | partly | **analysis → `legacy/`** | the old web driver's JSON shape (`"FunctionSchema/<uuid>"`). Kept for the parity tests; removed when 0.2 freezes the call-tree wire |
| `inject.rs` (138) | yes; **writes files**, mints UUIDs | **analysis → `legacy/`**, used only by `pylspt-dev inject` | it breaks the rule that a library never writes a file; it must never be reachable from the driver |
| `show.rs` (62), `main.rs` (201), `src/bin/*` | — | **pylspt-dev** | |

"Parse text directly" is a real change, not a move: `defs.rs`, `qname.rs` and `edit.rs` read through `ruff_db`'s salsa database today. It is the
work the 06-drivers §7 table already schedules for Step 2 ("outline function; `plan_ids` keyed per def, returning text + span"). Doing it as the
creation of `pylspt-syntax` is the same work, done once.

**[check]** while moving: the analysis crate may want the syntax crate's outline for a file it already parsed through salsa. Give `pylspt-syntax` a
function that takes an already-parsed `&ModModule` next to the one that takes text, so the analysis crate does not parse twice.

## 6. The public API: how a library stays easy to depend on

The driver, and anyone outside V-NOC, only ever touches `lib.rs`. Four habits keep that surface stable while the inside changes freely:

1. **One facade.** `lib.rs` re-exports what callers need, so no caller ever names a ruff crate (this is what ends the driver's three direct ruff
   dependencies, review R4):
   ```rust
   // crates/pylspt-syntax/src/lib.rs
   pub use ruff_text_size::{TextRange, TextSize};
   pub use outline::{outline, Def, DefKind, Outline};
   pub use edit::{plan_id, IdEdit};
   pub use header::header_slot;
   mod outline; mod ids; mod edit; mod header; mod lines; mod positions;   // private: free to change
   ```
2. **Modules are private by default.** Today `lib.rs` is `pub mod …` for every file, so every function is public API and any rename can break the
   driver. Make them `mod`, and export on purpose.
3. **Returned types are `#[non_exhaustive]`.** Then adding a field or an enum variant is not a breaking change.
4. **No policy inside.** The library never invents an ID, never names `FileID` or `vnoc`, never reads a config file. The caller passes the text
   (06-drivers §7). The vendored patch file `types/vnoc.rs` and its `pub mod vnoc;` line are renamed to `types/pylspt.rs` / `pub mod pylspt;` for the
   same reason: the analyzer should not know which product uses it.

> **🦀 Rust note: `#[non_exhaustive]`.**
> ```rust
> #[non_exhaustive]
> pub struct Def { pub qualname: String, pub kind: DefKind, pub range: TextRange, pub id_text: Option<String> }
> ```
> Outside the crate, nobody can build a `Def` with `Def { … }` or match on it without `..`. In return, the crate may add a field (`docstring_range`,
> say) in a minor release without breaking any caller. On an enum, callers must write a `_ =>` arm, so a new `DefKind::TypeAlias` does not break them
> either. This is exactly how S15 ("`kind` becomes an open set") is kept on the Rust side.

## 7. Tests contributors can add in five minutes

The cheapest contribution is "this Python file gives the wrong outline". Make that a file drop, not a code change:

```
crates/pylspt-syntax/tests/
├── fixtures/
│   ├── nested_locals.py
│   ├── decorators.py
│   ├── syntax_error_mid_file.py
│   └── emoji_in_strings.py
├── snapshots/                   written by the test, reviewed in the pull request
│   └── outline__nested_locals.snap
└── outline.rs                   ONE test that runs every file in fixtures/
```

> **🦀 Rust note: snapshot tests with `insta`.** Instead of writing the expected value by hand, the test prints its result and `insta` compares it
> with a file:
> ```rust
> #[test]
> fn outlines() {
>     insta::glob!("fixtures/*.py", |path| {
>         let text = std::fs::read_to_string(path).unwrap();
>         insta::assert_yaml_snapshot!(pylspt_syntax::outline(&text));
>     });
> }
> ```
> A contributor adds `fixtures/walrus.py`, runs `cargo insta review`, looks at the new snapshot, accepts it, and commits both files. A later change
> that alters any outline shows up as a diff of the `.snap` file in the pull request, which a reviewer can read without running anything.

Two more tests that guard the things users would lose data over:

| Test | Checks |
|---|---|
| **round trip** | for every fixture and every def: apply `plan_id`'s edit, outline again, the ID is read back, and nothing outside the edit's span changed |
| **never panics** | the outline of every fixture cut at every 50th byte (a half-typed file) returns, never panics. Later: a `cargo fuzz` target on the same function |

CI for pylspt, split so the common case is fast:

```yaml
jobs:
  syntax:    cargo test -p pylspt-syntax && cargo clippy -p pylspt-syntax -- -D warnings     # seconds; every push
  analysis:  cargo test -p pylspt-analysis                                                    # minutes; every push, can run in parallel
  docs:      cargo doc --no-deps -p pylspt-syntax -p pylspt-analysis                         # the public API builds its docs
```

## 8. Versions, releases, and changes that touch two repos

**Rename the GitHub repo `rust_python_parser` → `pylspt`.** It is the name in every plan, `Cargo.toml` and conversation; GitHub redirects the old URL,
so nothing breaks the day you do it. Then update the URL in `drivers/python/Cargo.toml`.

**Pin by tag, not by bare commit.** One version for the whole workspace, a tag per release:

```toml
# vnoc/drivers/python/Cargo.toml
pylspt-syntax = { git = "https://github.com/yaredtsy/pylspt", tag = "v0.3.0" }
```

A tag says what it is in a diff (`v0.3.0 → v0.4.0`); a hash does not. Cargo still records the exact commit in `Cargo.lock`, so a build is exactly as
reproducible as with `rev`. While pylspt is 0.x, a breaking API change bumps the middle number (`0.3 → 0.4`) and has a line in `CHANGELOG.md` saying
what to change.

> **🦀 Rust note: how a git dependency finds a crate in a workspace.** `{ git = "…", tag = "…" }` downloads the repo at that tag and then searches it
> for a package **with the name you asked for**, wherever it sits. So `pylspt-syntax` is found in `crates/pylspt-syntax/` without saying so, and the
> driver can depend on `pylspt-analysis` from the same URL and tag in 0.2. The repo is downloaded once for both.

**A change that touches both repos** (a new field the driver needs), in four steps a contributor can follow:

```
1. local      drivers/python/.cargo/config.toml (gitignored):
                [patch."https://github.com/yaredtsy/pylspt"]
                pylspt-syntax = { path = "../../../pylspt/crates/pylspt-syntax" }
              work on both; run both test suites
2. pylspt     pull request → merge → tag v0.4.0 → CHANGELOG line
3. vnoc       pull request: delete nothing from config.toml (it is gitignored), change the tag in Cargo.toml, `cargo update -p pylspt-syntax`
4. CI         vnoc's CI builds against the tag, never against a local path
```

`scripts/bump-pylspt.sh <tag>` in vnoc can do step 3 (edit the tag, `cargo update`, run the driver's tests) so it is one command.

## 9. The vendored ruff: updating it on purpose

pylspt vendors a trimmed copy of ty's dependency closure, because the call-tree work needs ty internals that are only reachable from inside
`ty_python_semantic` (decided in `pylspt/plan/01-crates/04-public-vs-private-api.md`; not reopened here). That is the right call, and it has one
cost: someone must be able to update it without you.

- **`vendor/ruff/PATCHES.md`** records the upstream commit (`ac201b8`), the list of crates in the closure, and every edit made to them. Today the
  edit is one line in `ty_python_semantic/src/types.rs` plus one new file. If a patch is not in this file, it does not exist.
- **`scripts/update-ruff.sh <new-commit>`**: clone upstream at that commit, recompute the closure from `ty_python_semantic`'s dependencies, copy
  those crates over `vendor/ruff/crates/`, re-apply `PATCHES.md`, copy the crates.io versions ruff pins into the root `[workspace.dependencies]`
  (salsa first), then `cargo test --workspace`.
- **When:** on purpose, at most once per pylspt release, in a pull request of its own that changes nothing else. Then a failing test after the
  update points at ruff, not at your code.
- `pylspt-syntax` depends on the three smallest crates of the closure (parser, AST, text size), which change rarely upstream. A ruff update
  mostly touches `pylspt-analysis`.

## 10. Add, update, remove: the three everyday jobs

| Job | Steps | Files outside the language's own folder |
|---|---|---|
| **add a language** | copy the folder shape of §3 · fill `DECISIONS.md` · tier 1 · conformance · one line in `scripts/package.mjs` · one entry in the CI matrix | 2 (packaging script, CI) |
| **update a language** | change it in its folder; for Python, a pylspt tag bump (§8) | 0 (Python: `Cargo.toml` tag) |
| **update the protocol** | change `crates/vnoc-driver-protocol`; every driver that does not compile is fixed in the same pull request; additive changes need no driver change (06-drivers §3d) | — |
| **move an analyzer out** (it now meets §2's conditions) | `git filter-repo --subdirectory-filter drivers/<lang>/analysis` into a new repo (keeps its history) · add README, CHANGELOG, CI · tag `v0.1.0` · in vnoc delete `drivers/<lang>/analysis/` and point `driver/Cargo.toml` at the git tag | 0 |
| **remove a language** | delete `drivers/<lang>/` · remove its packaging line and CI entry · release note | 2 |

What removing a language does to users: their `.vnoc/` data for it stays (it holds only the opaque `qualname` and `kind` strings), and those nodes
show as "no driver for this file", not as Missing (S16). Adding the driver back brings them back. So a language can be dropped from a release
without anyone losing a doc.

## 11. The order to do it in

Every step leaves both repos building and every test passing. Steps 1–2 are housekeeping with no code change; do them now, they cost an hour.

| Step | Repo | What | Proof |
|---|---|---|---|
| **1** | pylspt | commit or remove `run-jarvis.sh` / `cli-wire.json` (already a 06-drivers §7 checklist item); move `plan/ tutorial/ experience/` to `docs/`; `scripts/run-on-project.sh`; `out/` in `.gitignore`; README; rename the GitHub repo | `git status` clean; links in the notes still work |
| **2** | pylspt | `vendor/ruff/PATCHES.md`; rename the patch `vnoc.rs` → `pylspt.rs` | `cargo test` passes |
| **3** | pylspt | virtual workspace; `crates/pylspt-analysis` = today's `src/` moved unchanged; `crates/pylspt-dev` = the binaries, merged into subcommands | `cargo test --workspace` passes; same CLI output on `run-on-project.sh` |
| **4** | pylspt | `crates/pylspt-syntax`: `outline`, `ids`, `edit`, `header`, `lines`, `positions` on the plain parser; the facade (§6); snapshot + round-trip tests (§7); `pylspt-analysis` uses it | `cargo test -p pylspt-syntax` runs without building ty; tag `v0.3.0` |
| **5** | vnoc | the Python driver depends on `pylspt-syntax` by tag; the three ruff dependencies go; `outline.rs` shrinks to a type conversion | the driver's tests and the conformance test pass; the release build is smaller and faster to compile **[measure both]** |
| **6** | vnoc | `drivers/README.md` with §2's rule and §3's folder shape, so the next contributor starts from it | — |

Step 4 is the 06-drivers §7 "Step 2" row (outline function, per-def `plan_ids` with text + span) done as a crate. Step 5 is that table's "Step 0"
row (re-export, git pin, drop the three dependencies). So this plan adds no new work to the MVP; it gives the work that was already planned a
place to land.

### What this changes elsewhere in the plan

| Where | Was | Becomes |
|---|---|---|
| [`05-step-by-step.md`](05-step-by-step.md) §5 | "`drivers/<lang>/`, its own Cargo workspace if it needs its own toolchain, otherwise a member of the root one" | the same, with the two-crate folder of §3 here |
| [`06-drivers.md`](../06-drivers.md) §7 | pylspt pinned by `rev`; "pylspt re-exports what the driver needs" | pinned by `tag`; the re-exports are `pylspt-syntax`'s facade |
| `vnoc/drivers/python/Cargo.toml` comment | `pylspt` by `rev` with three ruff lines | `pylspt-syntax` by `tag`, nothing else from that repo until 0.2 |

Draft rows for your log:

| Draft row | Decision |
|---|---|
| **L1** | A driver always lives in `vnoc/drivers/<lang>/`. Its analyzer is a separate crate in that folder, and moves to its own repo only when it meets one of §2's four conditions. |
| **L2** | pylspt becomes a workspace of `pylspt-syntax` (parser only), `pylspt-analysis` (ty, salsa, call trees) and `pylspt-dev` (one CLI). The 0.1 driver depends only on `pylspt-syntax`. |
| **L3** | pylspt's learning material moves to `docs/`; the repo is renamed `pylspt`; the driver pins it by tag. |
| **L4** | Every vendored-ruff edit is listed in `vendor/ruff/PATCHES.md`; ruff is updated by `scripts/update-ruff.sh`, in a pull request of its own. |

Back to: [`README.md`](README.md)
