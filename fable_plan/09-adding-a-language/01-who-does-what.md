# 09.01 Who does what

```
who does what
├── 1. the three programs
├── 2. the split, job by job
├── 3. one attach, followed through all three
├── 4. what is still Python-only today (fix once, before driver two)
└── 5. can an existing language server do the driver's job?
```

## 1. The three programs

```
VS Code
└── extension (TypeScript)         knows VS Code.  Knows no language, no file format of V-NOC.
      │  stdin/stdout
      ▼
    core `vnoc` (Rust)             knows V-NOC: files in scope, the index, IDs, .vnoc/.  Knows no language.
      │  stdin/stdout, one pipe per driver
      ├──► vnoc-driver-python      knows Python.  Knows nothing about V-NOC's storage, IDs or VS Code.
      ├──► vnoc-driver-<next>
      └──► …
```

The rule that keeps this clean: **each program is ignorant of the other two's subject.** When you are unsure where a piece of code belongs, ask which
of the three subjects it is about: VS Code, V-NOC, or the language.

## 2. The split, job by job

| Job | Extension | Core | Driver |
|---|---|---|---|
| Find the driver programs inside the VSIX (`bin/`) and tell the core where they are | ✅ | | |
| Start, restart and stop a driver; time out a stuck request | | ✅ | |
| Decide which files are in scope (`.gitignore`, `[index]` in `vnoc.toml`, size limits) | | ✅ | |
| Say which file extensions belong to a language | | | ✅ (in its manifest) |
| Send each file to the right driver | | ✅ | |
| Parse a file: classes, functions, their names and ranges | | | ✅ |
| Build the language's own name for a def (`Outer.<locals>.inner`) | | | ✅ |
| Count "the 2nd def with this name" (`occurrence`) | | ✅ | |
| Read an ID that is written in the code, **as text** | | | ✅ |
| Decide whether that text is a V-NOC ID, an old UUID, or the user's own | | ✅ | |
| Choose a new ID | | ✅ | |
| Choose **where in the text** an ID goes, and produce the edit | | | ✅ |
| Apply that edit to the document (undo stack, dirty state, save) | ✅ | | |
| Folder markers (`.vnoc-id`) | | ✅ | |
| Read and write `.vnoc/` | | ✅ | |
| Resolve a documented node: marker → anchor → Missing | | ✅ | |
| Watch files for changes; debounce; report them | ✅ | | |
| Compute a call tree | | | ✅ |
| Cache, page and cancel call-tree requests | | ✅ asks | ✅ does |
| Menus, sidebar, canvas, doc panel | ✅ | | |
| Show a menu entry only on files a driver handles | ✅ (from a list the core sends) | ✅ (builds the list) | ✅ (source of the list) |

Three things a driver must **never** do, whatever the language:

1. **Write a file.** It returns text edits; the extension applies them. (That is what makes undo work and keeps unsaved typing safe.)
2. **Print to stdout** anything that is not a protocol message. One stray `println!` from a library breaks the pipe.
3. **Invent an ID or read `.vnoc/`.** It is given the ID text and puts it where it is told.

## 3. One attach, followed through all three

"Add doc" on a function, seen as a relay:

```
extension   the user clicks ＋ on `parse_order`            → vnoc/attach { key, title, text, version }   to the core
core        which driver owns orders.py? → python
            pick a free ID
            vnoc/planIds { text, version, ids: [{ qualname, occurrence, id }] }                          to the driver
driver      parse the text, find the def, find the place for the marker
            → { edits, written, skipped }                                                               back to the core
core        workspace/applyEdit { edits }                                                               to the extension
extension   apply to the open document (VS Code checks the version)        → applied: true / false
core        write node.toml, write the doc file, update the index         → vnoc/indexChanged
extension   refresh the tree, open the doc
```

The driver's whole part is the three middle lines. It did not learn what a doc is, where it is stored, or what the ID looks like.

## 4. What is still Python-only today

These are the places where "Python" leaked out of the driver. Each is small. Do them **once**, before the second driver; after that the goal in the
README holds.

| Where | What is hard-coded | Make it |
|---|---|---|
| core `scan.rs` | `python_files`, extension `== "py"` | "files whose extension some driver's manifest claims" ([`03`](03-files-and-driver-selection.md) §2) |
| core start-up | one `driverPath` in `initializationOptions` | a folder of drivers, each with a manifest ([`03`](03-files-and-driver-selection.md) §1) |
| protocol crate | `DefKind` is exactly `function` or `class` | an open set: known kinds + any other string kept as is ([`02`](02-what-a-driver-must-do.md) §2) |
| extension `package.json` | `when: resourceLangId == python` on menus and buttons | `when: resourceLangId in vnoc.languageIds`, a context key the extension fills from the core's answer |
| extension tree + webview | icons chosen for "function" / "class"; the snake icon for files | icon by `kind`, with a generic one for unknown kinds; file icon from VS Code's own file-icon theme |
| webview code view | Shiki loads the Python grammar only | load the grammar for the file's language id on demand |
| driver ↔ core types | the driver builds JSON by hand | both sides use `vnoc-driver-protocol` (review D1) |
| tests | fixtures are Python only | the conformance test takes a driver path and a fixture folder ([`05`](05-step-by-step.md) §4) |

> **🧩 VS Code note: a `when` clause that follows the installed drivers.** A `when` clause can test membership in a list you control:
> ```ts
> // extension, after the core answered `initialize` with the languages its drivers handle
> vscode.commands.executeCommand("setContext", "vnoc.languageIds", ["python", "typescript"]);
> ```
> ```jsonc
> { "submenu": "vnoc.editorMenu", "when": "resourceLangId in vnoc.languageIds" }
> ```
> `setContext` creates (or updates) a named value that `when` clauses can read. The menu entry now appears on exactly the languages that have a driver,
> and `package.json` never names a language again.

## 5. Can an existing language server do the driver's job?

For most languages a good language server already exists (rust-analyzer, gopls, tsserver, clangd). It is tempting to use it instead of writing a
parser. What it can and cannot give you:

| V-NOC needs | Standard LSP request that is close | Fit |
|---|---|---|
| outline (tier 1) | `textDocument/documentSymbol` | good: names, kinds, ranges, nesting. No "ID found in the code". |
| read / place markers (tier 2) | none | not available: this is V-NOC's own idea |
| call tree (tier 3) | `callHierarchy/outgoingCalls` | good where the server implements it; quality varies by language |

So a realistic shape for a language other than Python is a **thin driver that owns markers itself and borrows the rest**:

```
vnoc-driver-go
├── outline + markers      its own small parser (tree-sitter grammar, or the language's parser library)
└── call trees             starts gopls as a child process and translates callHierarchy answers      ← idea, not tested
```

This is a direction, not a decision: nobody has tried it yet, and starting someone else's language server has costs (start-up time, memory, finding the
binary on the user's machine). What matters now is that the protocol does not rule it out, and it does not: the core only sees `vnoc/…` answers.

Tier 1 with tree-sitter alone is the cheap way to make a language *appear* in V-NOC, and it is enough for docs attached by name.

Next: [`02-what-a-driver-must-do.md`](02-what-a-driver-must-do.md)
