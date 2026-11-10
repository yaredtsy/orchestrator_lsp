# 09.05 Step by step: adding driver number N

Follow this top to bottom. Each step ends with something you can run.

```
steps
├── 0. before the first new driver: make the product language-blind   (once)
├── 1. fill in the decision sheet
├── 2. tier 1: the language appears
├── 3. tier 2: docs survive refactors
├── 4. the conformance test
├── 5. packaging
├── 6. tier 3: call trees
└── 7. what you must not need to touch
```

## 0. Once, before the first new driver

The table in [`01-who-does-what.md`](01-who-does-what.md) §4. In order:

- [ ] the Python driver uses the `vnoc-driver-protocol` types (review D1)
- [ ] `DefKind` becomes an open set
- [ ] manifests: the Python driver's build writes `vnoc-driver-python.json`; the core reads `bin/*.json` and routes by extension
- [ ] `scan.rs` loses the word "python"
- [ ] the extension sets `vnoc.languageIds`; `package.json` loses `resourceLangId == python`
- [ ] icons by `kind`, with a generic fallback; Shiki grammar by language id
- [ ] the conformance test exists and the Python driver passes it (§4)

**Proof that step 0 is done:** write a fake driver (§2 shows how small it is) for a made-up extension, drop it in `bin/`, and see its files in the
tree without editing any file outside the driver's folder.

## 1. The decision sheet

Create `drivers/<lang>/DECISIONS.md` and answer these **before writing code**. Most of them are frozen once users have docs on that language.

```markdown
# <language> driver: decisions

## Files
- extensions claimed:                  e.g. go
- file names claimed:                  e.g. (none)
- VS Code language ids:                e.g. go
- extensions deliberately NOT claimed: e.g. go.mod, go.sum (config / lock files)

## Definitions                                                    FROZEN after release
- what is a def (kinds):               e.g. function, method, struct, interface
- what is not:                         e.g. local closures, struct fields, constants
- qualname rule:                       e.g. `Type.Method` for methods, `name` for functions; no package prefix
- separator for nesting:               e.g. `.`
- can two defs share a qualname in one file? when?   e.g. `init` functions; build-tagged duplicates
- what `range` covers:                 e.g. doc comment + signature + body

## Markers                                                        FROZEN after release
- def marker: exact text and place:    e.g. `// vnoc: <ID>` as the last line of the doc comment, or alone above the def
- def with no doc comment:             e.g. insert one comment line above the def, same indentation
- file marker: exact text:             e.g. `// FileID: <ID>`
- things that must stay above it:      e.g. build constraints + their blank line; licence header
- when the driver refuses a file:      e.g. "Code generated … DO NOT EDIT." header; cgo-generated files
- formatter check: does <formatter> keep the marker in place?     (run it, paste the result)
- linter check: does <linter> complain with default rules?        (run it, paste the result)

## Call trees (tier 3)
- how calls are resolved:              e.g. own analysis / the language's compiler API / its language server
- environment input files:             e.g. go.mod, go.sum, go.work
- what "external" means:               e.g. anything outside the module in go.mod

## Implementation
- language the driver is written in:
- parser used:
- how positions are converted to UTF-16:
```

The two "check" lines under Markers are the ones people skip and regret. A marker the formatter moves is worse than no marker.

## 2. Tier 1: the language appears

A driver can be written in any language that can read stdin and write stdout. In Rust, the skeleton is small:

> **🦀 Rust note: the whole shape of a tier-1 driver.**
> ```rust
> // drivers/<lang>/src/main.rs  (sketch)
> use lsp_server::{Connection, Message, Response};
> use vnoc_driver_protocol as proto;
>
> fn main() -> anyhow::Result<()> {
>     let (conn, io_threads) = Connection::stdio();           // framed JSON-RPC on stdin/stdout
>     let (id, params) = conn.initialize_start()?;            // waits for `initialize`
>     let params: proto::InitializeParams = serde_json::from_value(params)?;
>     anyhow::ensure!(params.protocol == proto::PROTOCOL_VERSION, "unsupported protocol");
>     conn.initialize_finish(id, serde_json::to_value(my_initialize_result())?)?;   // then waits for `initialized`
>
>     for msg in &conn.receiver {
>         match msg {
>             Message::Request(req) if conn.handle_shutdown(&req)? => break,
>             Message::Request(req) => {
>                 let resp = match req.method.as_str() {
>                     proto::method::OUTLINE => outline(req.id.clone(), req.params),      // ← your language work
>                     _ => Response::new_err(req.id.clone(), -32601, "unknown method".into()),
>                 };
>                 conn.sender.send(Message::Response(resp))?;
>             }
>             _ => {}                                           // unknown notifications are ignored
>         }
>     }
>     io_threads.join()?;
>     Ok(())
> }
> ```
> `lsp_server` (the crate the Python driver uses) gives you the framing and the handshake. Everything language-specific is inside `outline`. Use
> `eprintln!` for logs, never `println!`.

- [ ] the skeleton answers `initialize` with `capabilities: { "outline": true }`
- [ ] `outline` returns defs for a clean file
- [ ] positions are UTF-16 (test: an emoji in a string **before** a def on the same line, and a non-ASCII identifier)
- [ ] a file with a syntax error still returns the defs around the error, flagged
- [ ] nested defs: `parent` and `qualname` are right two levels deep
- [ ] duplicates: two defs with the same name come out as two entries with the same `qualname`
- [ ] the manifest is generated by the build

*Done when:* open a project in that language → its files, classes and functions are in the sidebar and on the canvas; "Add doc" attaches by name (the
outlined link icon).

## 3. Tier 2: docs survive refactors

- [ ] `outline` reports `id` on defs and `file_id` for the file, as written
- [ ] `vnoc/planIds`; capability `ids.plan`
- [ ] `vnoc/planFileId`; capability `ids.file` (or leave it false and say why in the decision sheet)
- [ ] the **round-trip** test: plan → apply → outline shows the ID on the same def, nothing else changed
- [ ] the **no-damage** test over a folder of real files of that language: after plan + apply, every file still parses, and the diff is only marker lines
- [ ] the **formatter** test: plan + apply, run the standard formatter, outline again → every ID still on its def
- [ ] the refusal cases from [`04`](04-markers-and-unsafe-files.md) §3: generated header, shebang, syntax error at the top, CRLF file, BOM, no final newline

*Done when:* add a doc to a function, rename the function, move it to another file, restart VS Code → the doc is still on it (the solid link icon).

## 4. The conformance test

One program that checks any driver, so "does my driver follow the contract" is a command and not a reading exercise.

```
cargo test -p vnoc --test driver_conformance        with  VNOC_DRIVER=…/vnoc-driver-go  VNOC_FIXTURES=drivers/go/conformance
```

```
drivers/<lang>/conformance/
├── outline/
│   ├── basic.<ext>                 + basic.expected.json          (the outline you expect)
│   ├── nested.<ext>                + …
│   ├── unicode.<ext>               an emoji and a CJK identifier before a def
│   └── broken.<ext>                a syntax error in the middle
├── ids/
│   ├── no-doc-comment.<ext>        + .expected.<ext>  (the text after applying the plan)
│   ├── has-doc-comment.<ext>
│   └── …
└── file-id/
    ├── shebang.<ext>   crlf.<ext>   generated.<ext> (expects a refusal)   …
```

What the test does that is the same for every language (no per-driver code):

| Check | Tier |
|---|---|
| handshake; a wrong protocol number is refused; the manifest agrees with the handshake | all |
| an unknown request → `-32601`; an unknown notification is ignored; the driver is still alive after both | all |
| any byte on stdout outside a frame fails the run | all |
| `shutdown` + `exit` ends the process; closing stdin ends the process | all |
| each `outline/*` file gives its `expected.json`; with `text` and with `root` + `file` | 1 |
| every range is inside the text and UTF-16-consistent (it re-slices the text and compares with `name`) | 1 |
| each `ids/*`: apply the edits → equals the expected text; outline of the result reports the IDs (round trip) | 2 |
| each `file-id/*`: same, or the expected refusal | 2 |
| line endings, BOM and final newline of every result equal the input's | 2 |

Run it in CI for every driver in the repo. A driver written outside this repo runs the same test against its own fixtures.

## 5. Packaging

- [ ] the driver's folder: `drivers/<lang>/` (its own Cargo workspace if it needs its own toolchain, as Python does; otherwise a member of the root one),
      with the analyzer and the driver as two crates: [`06-organizing-language-code.md`](06-organizing-language-code.md) §3
- [ ] `scripts/package.mjs`: build the driver for the target platform, copy the program **and its manifest** into `extension/bin/`
- [ ] CI: add it to the build matrix (same four platforms as the core)
- [ ] check the VSIX size before and after; note it in the release notes
- [ ] if the driver needs a runtime on the user's machine (a JVM, Node, the language's own toolchain): say so in the decision sheet, detect its absence
      at start, and fail with a sentence a person can act on. Prefer no runtime dependency at all.

That list contains **one** edit outside the driver's folder: the line in the packaging script (and the CI matrix).

## 6. Tier 3: call trees

Only after 0.2 has defined the final wire shape on the Python driver. Then:

- [ ] `vnoc/environmentInputs`
- [ ] `vnoc/callTree` with a budget, a frontier and stable `{ file, qualname, occurrence }` targets
- [ ] `vnoc/expand` with a token; paging of long child lists
- [ ] `$/cancelRequest` really stops the work
- [ ] conformance fixtures: a small multi-file project with a known call tree, a recursive function, an external call, an unresolvable call

Design the analysis around "stop at a budget and remember where you stopped" from the start ([`02`](02-what-a-driver-must-do.md) §4 and §6).

## 7. What you must not need to touch

If adding your driver made you edit any of these, stop: something from step 0 is missing, and the fix belongs there, not in a special case for your
language.

```
crates/vnoc/src/**          the core            (no `if language == …`)
crates/vnoc-domain/**       IDs, node.toml
extension/src/**            the extension
extension/package.json      menus, when-clauses
webview/src/**              canvas, cards, doc panel
.vnoc/ formats              nothing per language is stored there, except the opaque `qualname` and `kind` strings
```

The only shared file a driver may add to is the list of **known kinds** (for a nicer icon than the generic one), and that is optional.
