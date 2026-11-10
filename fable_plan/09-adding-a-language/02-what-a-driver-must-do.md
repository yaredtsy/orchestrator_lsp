# 09.02 What a driver must do

The wire spec is `plan_vs/07-drivers/01-driver-protocol.md`. This file is the same contract read from the driver author's chair: what you must
implement, in what order, and the rules that are easy to get wrong.

```
the contract
├── 1. always: the process rules
├── 2. tier 1  STRUCTURE     initialize · vnoc/outline
├── 3. tier 2  MARKERS       reading IDs · vnoc/planIds · vnoc/planFileId
├── 4. tier 3  CALL TREES    vnoc/callTree · vnoc/expand · paging · cancel · vnoc/environmentInputs
├── 5. speed and size: what the core expects
└── 6. where the Python driver stands
```

A driver announces what it has in `capabilities`. The core checks before it calls, so a tier-1 driver is a complete, valid driver.

## 1. Always: the process rules

| Rule | Why |
|---|---|
| Read framed JSON-RPC from **stdin**, write it to **stdout**. Logs go to **stderr** only. | stdout *is* the protocol |
| `initialize` → refuse a `protocol` number you do not speak (`-32600`, say why). Answer `name`, `version`, `capabilities`. | old and new parts can meet safely |
| Do no work before the `initialized` notification. | same as LSP |
| `shutdown` → stop taking work, answer `null`. `exit` or stdin closing → leave. | the core must be able to stop you |
| Unknown notification → ignore. Unknown request → `-32601`. **Never exit because of a message you do not understand.** | a newer core must not kill an older driver |
| Ignore JSON fields you do not know. | additions never break you |
| Never write a file. Never print a stray byte on stdout. | [`01`](01-who-does-what.md) §2 |
| A crash may end the process. The core restarts you and retries nothing silently. | no half-states |

## 2. Tier 1: STRUCTURE

With only this, the language appears in the sidebar and on the canvas, and docs can be attached by name.

### `vnoc/outline`

Input: `{ text }` (an unsaved buffer) **or** `{ root, file }` (a path you read). Output: a flat list of definitions.

```json
{ "defs": [ { "qualname": "OrderService.parse_order", "name": "parse_order", "kind": "function",
              "parent": "OrderService", "parent_kind": "class",
              "range": { "start": {"line": 4, "character": 4}, "end": {"line": 12, "character": 30} },
              "params": ["self", "raw"], "id": null, "syntax_error": false } ],
  "syntax_errors": [ ] }
```

The rules, and the reason for each:

| Rule | Reason |
|---|---|
| **Flat list, in source order, parents before children.** | the core rebuilds the tree from `parent`; you do no nesting work |
| **`qualname` is your language's own way to name the thing**, and you never change the rule once released. | it is stored in `node.toml` and is part of the ID's hash input. A changed rule orphans every doc of that language. |
| `qualname` is unique inside the file **except** for true duplicates (two defs with the same name in the same scope). Do not number them yourself. | the core counts `occurrence` |
| `qualname` does not contain the file path. | the path is stored separately; a moved file must keep its qualnames |
| **Positions: 0-based line, `character` counted in UTF-16 code units.** | identical to VS Code. Most parsers give byte offsets, so convert at your boundary. Test with an emoji and a CJK character before the def. |
| `range` covers the whole definition: from its first decorator / attribute / doc comment to its last line. | the card shows exactly this slice; "is the cursor inside this def" uses it |
| **A file with syntax errors still returns what you can**, and sets `syntax_error: true` on defs whose lines contain an error. When unsure, flag more, never fewer. | people open V-NOC while typing |
| `kind` says what it is. | icons, and the Relink picker |

**About `kind`.** Today the protocol crate allows exactly `function` and `class`. A second language needs more. Make it an open set before driver two:

```
known kinds   function · method · class · struct · interface · trait · enum · module · namespace · type
anything else kept as the string it is; the UI shows a generic icon
```

> **🦀 Rust note: an enum that keeps what it does not know.**
> ```rust
> #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
> #[serde(rename_all = "lowercase")]
> pub enum DefKind {
>     Function, Method, Class, Struct, Interface, Trait, Enum, Module,
>     #[serde(untagged)]
>     Other(String),          // any other text lands here instead of failing to parse
> }
> ```
> `#[serde(untagged)]` on the last variant means "if no name above matched, take the raw string". Your `IdInCode::Other` in `vnoc-domain` already uses
> this idea. Without it, a new driver that says `"kind": "protocol"` would make the whole outline fail to parse in an older core.

**What counts as a def is your decision,** per language. A good default: things a person would want to write a note on and that have a stable name.
Functions, methods, classes and their equivalents: yes. Local variables, lambdas without a name, fields: no. Write the decision down
([`05`](05-step-by-step.md) §1); it is hard to change later because adding a kind shifts nothing, but removing one orphans docs.

## 3. Tier 2: MARKERS

With this, docs survive renames and moves. Capabilities: `ids.plan` (definitions), `ids.file` (the file itself). A driver may have one without the
other.

### Reading

In `vnoc/outline`, fill `id` with the ID text you found on that def, **exactly as written**, or `null`. For the file's own marker, fill the top-level
`file_id`. Do not validate, normalise or interpret it. The core decides what it is.

### `vnoc/planIds`

```json
request:  { "text": "…", "version": 7, "ids": [ { "qualname": "…", "occurrence": 1, "id": "3RGJ…", "replace_existing": false } ] }
response: { "version": 7, "edits": [ … ], "written": [ … ], "skipped": [ … ], "unmatched": [ … ], "text_hash": "…" }
```

| Rule | Reason |
|---|---|
| Plan on the `text` you were given, never on the file on disk. Echo `version`. | the buffer may have unsaved typing; the editor drops a plan made for an older version |
| `edits` are in the coordinates of the **old** text; they must not overlap. | VS Code applies them all against the original |
| `written[].span` and `.text`: what was inserted and where it is in the **new** text. | removal later needs the exact text |
| A def you cannot safely edit goes to `skipped` with a reason a person can read. The other defs still get their IDs. | one broken function must not block the rest |
| A `(qualname, occurrence)` you cannot find goes to `unmatched`. | the text changed since the outline |
| **The round-trip rule:** apply your own edits, run your own outline on the result → every written ID is reported on the right def, and nothing else in the outline changed. | this one test catches most placement bugs |
| **The no-damage rule:** the new text parses, and differs from the old text only by the inserted marker text. No reformatting, no changed line endings, no touched indentation elsewhere. | V-NOC must be invisible in `git diff` except for the one line |

Where the marker goes is yours to design per language. The constraints are in [`04`](04-markers-and-unsafe-files.md).

### `vnoc/planFileId`

`{ text, version, id }` → `{ version, edits, written, text_hash }`, or an error / `skipped` when this file must not be touched. Same rules as above.
The placement rules (after a shebang, after an encoding line…) are in [`04`](04-markers-and-unsafe-files.md) §3.

## 4. Tier 3: CALL TREES

This is the 0.2 feature. The wire shape is only reserved today ("finished when M5 is designed"), so read this section as **requirements plus a
proposed shape**, not as a spec.

### What the UI will ask for

```
the user opens `checkout` on the canvas      → what does it call?            (one level, fast)
clicks ＋ on `checkout_order`                 → and what does that call?      (one more level, on demand)
clicks ＋ on a function with 300 callees      → show the first 50, "more…"    (a page)
clicks away while a slow one is running      → stop working on it            (cancel)
opens `checkout` again after a restart       → instant                       (cache; the core's job, with the driver's help)
```

### What that requires from a driver

| Need | Meaning for the driver | Proposed shape |
|---|---|---|
| **Bounded work** | Stop *computing* at a limit, not only stop *printing*. | request carries `budget: { max_depth, max_nodes, ms }` |
| **A frontier** | Say which nodes were cut, so the UI can draw a `＋`. | a cut node has `"more": { "token": "…" }` instead of `children` |
| **Resume** | Continue from a cut node without redoing the tree above it. | `vnoc/expand { token }` (or `{ target }` when the token is stale) |
| **Paging** | A long child list comes in pieces. | `children` + `"next": "…"`; `vnoc/expand { token, cursor }` |
| **Cancel** | `$/cancelRequest { id }` really stops the analysis; answer `-32800`. | already in the wire spec §3 |
| **Stable node names** | A callee is named by `{ file, qualname, occurrence }`, the same triple the outline gives. | so the core can join a callee to an indexed node and to its docs |
| **External leaves** | A call into a library or the standard library is a leaf with a name, not an error. | `"external": { "package": "…", "name": "…" }` |
| **Unsaved text** | The entry file may be given as `text`. | same `text` or `root` + `file` choice as `outline` |
| **Honest uncertainty** | A call that cannot be resolved is reported as unresolved, with the call site. | a `marks` entry; never guess silently |

A token is opaque to the core: the driver may put anything in it (an index into a cache it keeps, or enough to recompute). It must stay valid while the
driver process lives and the file has not changed; after that the driver answers "stale" and the core falls back to `{ target }`.

### `vnoc/environmentInputs`

`{ root }` → a list of file globs (`pyproject.toml`, `package.json`, `go.mod`, lock files…) whose change means "resolution results may be different
now". The core hashes them and clears cached call trees when the hash changes. A driver with no such notion returns an empty list.

## 5. Speed and size: what the core expects

The core's timeouts and the UI's feel are built on these. They are targets taken from the Python driver's measurements (spikes S2/S3), not hard limits.

| Request | Target | Note |
|---|---|---|
| start-up to `initialize` answered | under 200 ms | the first tree is waiting for you |
| `outline` of one file | a few ms; under 50 ms for a 5,000-line file | it runs once per file at start, so ×1,000 |
| `planIds` on a buffer | under 50 ms | the user is waiting on a click |
| first project-wide request (call tree) | may pay a one-time load; say so in the log | Python: ~130 ms once |
| any request | answer or fail within the core's timeout (10 s today) | else the core kills and restarts you |

Also: filter your own logging by default (a type checker can write tens of MB of trace), and keep the binary small (it ships in every VSIX for that
platform).

## 6. Where the Python driver stands

So you can see what "done" means on a real driver, and what is still open on the reference one.

| Contract item | Python driver (`drivers/python` + pylspt) |
|---|---|
| process rules, handshake | ✅ |
| `outline`: defs, `<locals>` qualnames, UTF-16 ranges, syntax-error flags | ✅ (the walk should move into pylspt, [`06-drivers.md`](../06-drivers.md) §7) |
| reading `ID:` from docstrings | ✅ |
| `file_id` in the outline | ❌ to add |
| manifest with file extensions | ❌ to add ([`03`](03-files-and-driver-selection.md) §1) |
| `planIds` | 🟡 works in spike S3; to port; pylspt's `plan_ids` needs the per-def form |
| `planFileId` | 🟡 prototyped in the spike's adapter |
| `callTree`: computes the tree | ✅ in pylspt (`src/interp/`), not wired into the product driver |
| **bounded work** | ❌ `render_depth` shortens the *output*; the solver still solves the whole tree. Cheap on small repos (~100 ms on httpx). |
| **frontier / resume / paging** | ❌ not in pylspt. Deferred as one batch with cancel and caching (VS-D69). |
| **external cancel** | ❌ pylspt has a deadline only |
| cache across requests | ❌ |
| unsaved text for call trees | ❌ to check |
| `environmentInputs` | 🟡 the list is known; can live in the adapter |

The four ❌ rows in bold are one piece of work inside pylspt and are the real cost of 0.2. A second language's driver will face the same four; design
its analysis around "stop at a budget and remember where you stopped" from the first day, because adding it afterwards is what made it a batch job
here.

Next: [`03-files-and-driver-selection.md`](03-files-and-driver-selection.md)
