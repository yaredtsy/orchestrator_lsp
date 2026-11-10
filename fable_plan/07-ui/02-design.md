# 07.02 UI design plan

```
ui
├── 1. the surfaces (where V-NOC appears in VS Code)
├── 2. the sidebar
├── 3. the canvas
│   ├── layout of the panel
│   ├── the node card
│   ├── writing the first doc
│   └── the doc panel (read here, edit in a tab)
├── 4. looking native: theme, icons, spacing
├── 5. code in a card: Shiki for reading, Monaco for editing
└── 6. what is in 0.1, and what waits
```

## 1. The surfaces

```
VS Code window
├── Activity bar          the V-NOC icon                                         ✅ exists
│   └── Side bar
│       ├── STRUCTURE     tree: folders ▸ files ▸ classes ▸ functions            ✅ exists
│       └── MISSING       documented nodes that cannot be found  (hidden when empty)   0.1
├── Editor area
│   ├── V-NOC canvas      a webview tab                                          ✅ exists (basic)
│   │   └── doc panel     on the right of the canvas: the doc, rendered, read-only     0.1
│   └── a doc (editing)   a normal Markdown editor tab, opened by the panel's Edit     0.1
├── Context menus         on tree items and on Explorer items                           0.1
├── Command palette       "V-NOC: …" commands                                    ✅ 3 exist
└── Status bar            (nothing in 0.1)
```

> **🧩 VS Code note: native views vs webviews.** The sidebar tree is a **native** `TreeView`: you give VS Code data (`getChildren`, `getTreeItem`) and it
> draws, themes, virtualises and makes it keyboard-accessible for you. The canvas is a **webview**: you draw everything yourself in HTML. Rule of thumb:
> use a native view whenever the thing is a list or a tree, and a webview only for what VS Code cannot draw (a graph). That is why "Missing" should be a
> second native view and not a panel inside the canvas.

## 2. The sidebar

```
STRUCTURE                                    ⟳
▾ 📁 src
  ▾ 📁 billing                    📄1            ← doc count, shown as the item's description
    ▸ 🐍 invoice.py
  ▾ 🐍 orders.py
    ▾ ◆ OrderService
        ƒ parse_order             📄2
        ƒ validate                               ← hover: [⌖ focus on canvas] [＋ attach doc]

MISSING (2)
  ƒ parse_legacy   src/old.py · 1 doc            ← click: Relink…    context menu: Relink · Remove
  📁 src/reports   · 2 docs
```

| Element | 0.1 behaviour |
|---|---|
| click a file / class / function | opens the code at that line (exists) |
| hover buttons | **Focus on canvas** (exists) · **Attach doc** (new) |
| context menu | Add doc · Focus on canvas · Open docs ▸ (list; opens the canvas with the doc panel) |
| description text | `📄 n` when the node has docs; `syntax error` (exists) |
| MISSING view | appears only when something is missing (`"when"` clause on a context key the extension sets) |
| Explorer context menu | "V-NOC: Attach doc" on files and folders, so documenting does not require opening the V-NOC view |

A node found only "by name" gets a tooltip: *"Attached by name. It will be lost if renamed. Write ID to make it permanent."*

## 3. The canvas

### The panel

```
┌────────────────────────────────────────────────────────────────────────────┐
│ V-NOC   my-project · 12 folders · 87 files · 430 functions          ⟳      │  bar
│ All code › src › billing › invoice.py                                      │  breadcrumbs = the focus stack   ✅
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌──────────┐      ┌──────────────┐      ┌───────────────────┐            │
│   │ 📁 src   ├──────┤ 📁 billing   ├──────┤ 🐍 invoice.py     │   …        │  cards, left to right by depth    ✅
│   └──────────┘      └──────────────┘      └───────────────────┘            │
│                                                                    ┌─────┐ │
│                                                                    │ map │ │  minimap, zoom controls           ✅
└────────────────────────────────────────────────────────────────────┴─────┘─┘
```

The tree layout you have (depth to the right, one row per node) is the right one for 0.1: it is predictable and it matches the sidebar. A graph layout
library (dagre, elk) becomes necessary in 0.2, when call edges cross the tree.

### The node card

```
┌────────────────────────────────────────────────────┐
│ ▸  ƒ  parse_order              🔗   📄 2    </>    │   header
├────────────────────────────────────────────────────┤
│   (self, raw)                                      │   body:  signature / children count
│                                                    │          or the code (when </> is on)
├────────────────────────────────────────────────────┤
│ src/orders.py:42                     ⌖    ↗    ＋  │   footer
└────────────────────────────────────────────────────┘
```

| Part | Element | Meaning | State |
|---|---|---|---|
| header | `▸` | expand / collapse children | ✅ |
| | kind icon + name | folder, file, class, function | ✅ |
| | `🔗` link icon | **solid** = found by marker · **outlined** = found by name · absent = no ID | 0.1 |
| | `📄 n` | number of docs; **shown only when n ≥ 1**; click opens the doc panel on this node | 0.1 |
| | `</>` | show the code in the body | ✅ |
| body | one of two contents | summary · code | code ✅ |
| footer | path:line | where it is | ✅ |
| | `⌖` | focus (same as double-click) | 0.1 (double-click exists) |
| | `↗` | go to code in the editor | 0.1 |
| | `＋` | add a doc: **always shown**, also on a node with no docs yet | 0.1 |

Visual states of the whole card: normal · selected (focus ring) · syntax error (exists: `broken`) · Missing cards are **not** on the canvas (they have no
place in the tree); they live in the MISSING view.

Card height is a constant today (`ROW_H`, `CODE_H`). The docs moved to the panel, so the card keeps two body modes and the constants still work.

Later (not 0.1): the web version shows the first lines of the doc in the card body. That is a good summary mode once docs exist; it needs measured card
heights (React Flow reports each node's rendered size; feed those into `layout()` instead of constants).

### Writing the first doc

A node with no docs has no `📄` badge, so the way in is the `＋`. There are five places to start, and all of them run the same command
(`vnoc.addDoc`, with the node as its argument):

```
card footer            ＋
doc panel              "＋ New doc…" in the dropdown, and the button in the empty state
sidebar tree           hover button ＋ and the context menu "Add doc"
Explorer               context menu "V-NOC: Add doc" on a file or folder
Command palette        "V-NOC: Add doc to the function at the cursor"
```

What happens after the click:

```
1. title?            VS Code input box, pre-filled with "Overview" for a node's first doc
                     (Esc cancels; nothing has been written yet)
2. first time in     one question, once per project:
   this project        "V-NOC will add an ID line to parse_order's docstring and create a .vnoc/ folder in this repository."
                       [Add doc]   [Add without ID]   [Cancel]
                     "Add without ID" sets `[ids] default = "none"` in vnoc.toml. Later docs do not ask again.
3. the core attaches   ID → marker in the code → node.toml → overview.md with frontmatter and "# Overview"
                       (04-storage/03-writing-safely.md §3)
4. the UI shows it     · the card gets 🔗 and 📄 1
                       · the doc panel opens on the new doc
                       · the .md opens in an editor tab with the cursor under the heading, ready to type
```

The second doc on the same node is the same flow without step 2, and the title box starts empty.

Step 2 exists because the first attach is the only moment V-NOC edits the user's source code without being asked in so many words. One clear question,
once, is cheaper than a surprised user looking at `git diff`.

The title input uses `vscode.window.showInputBox` (native) rather than a text field inside the webview: less code, and it behaves like every other VS Code
prompt.

### The doc panel

```
┌ V-NOC canvas ─────────────────────────────┬──────────────────────────────────┐
│ All code › src › orders.py                │ Overview ▾          ✎ Edit    ✕  │  ▾ = this node's docs + "＋ New doc…"
├───────────────────────────────────────────┤ ƒ parse_order · src/orders.py:42 │  which node the doc belongs to
│                                           ├──────────────────────────────────┤
│   ┌────────┐      ┌──────────────┐        ║                                  │
│   │ 📁 src ├──────┤ 🐍 orders.py │   …    ║  # Why we parse twice            │  the Markdown, rendered
│   └────────┘      └──────────────┘        ║                                  │  read-only
│                                           ║  The upstream API sends…         │
│                                           ║                                  │
└───────────────────────────────────────────┴──────────────────────────────────┘
                                            ║ = drag to resize
```

| Part | Behaviour |
|---|---|
| open | click `📄 n` on a card · "Open docs" in the sidebar · after adding a doc |
| which node | the node whose `📄` was clicked. While the panel is open, selecting another card switches the panel to that card's docs. |
| `▾` dropdown | the docs of this node by title · last entry "＋ New doc…" |
| `✎ Edit` | opens the `.md` in a normal editor tab (`showTextDocument(uri, { viewColumn: ViewColumn.Beside })`) |
| `✕` | closes the panel; the canvas takes the full width again |
| empty state | a node with no docs: "No docs on `parse_order` yet." and a **Write the first doc** button (runs the flow above) |
| width, open/closed, chosen doc | webview view state (`vscode.setState`), so it survives hiding and showing the tab |

**Read here, edit in a tab.** The panel never writes. Editing happens in a VS Code Markdown tab, which gives you undo, save, search and spell-check
extensions without writing editor code. The panel is a live preview of that tab:

```
editor tab (the .md)                    extension                              webview panel
  typing ──► onDidChangeTextDocument ──► debounce ~150 ms ──► docText { uri, text } ──► render
  (no tab open: the extension reads the file, and sends it again when the file watcher reports a change)
```

> **🧩 VS Code note: why the text comes from the extension.** A webview cannot read files. The extension reads the doc and posts the text in. If the doc
> is open in an editor, read it with `vscode.workspace.openTextDocument(uri)`: that returns VS Code's in-memory copy, **including unsaved typing**, so the
> panel updates while you type and not only when you save.

Rendering rules:

- Strip the frontmatter (`---` … `---`) before rendering; show the title from the first `# heading`.
- Use a small Markdown renderer (`markdown-it`, or `react-markdown`) with **raw HTML turned off**. Docs arrive by `git pull` from other people, so treat
  them as untrusted input: no `<script>`, no inline HTML at all.
- Code blocks are coloured by Shiki, which is already in the bundle.
- A link is not followed inside the webview. The click goes to the extension, which opens `http(s)` links in the browser and relative links as files.
- Images with relative paths do not load in 0.1 (they need `webview.asWebviewUri` and a wider `localResourceRoots`). Show the alt text.

**What 0.3 changes:** the panel gets an editor of its own, so typing happens in place like the web version. The layout, the dropdown and the messages
stay; only the read-only body is replaced.

## 4. Looking native

Three habits make a webview feel like part of VS Code instead of a web page inside it:

1. **Colours only from VS Code's theme variables.** VS Code injects hundreds of CSS variables into every webview (`--vscode-editor-background`,
   `--vscode-foreground`, `--vscode-focusBorder`, `--vscode-badge-background`…). Use them and the canvas follows any theme, including high contrast, with no
   work. Collect the ones you use in one place:
   ```css
   /* webview/src/tokens.css — the only file that names a --vscode-* variable */
   :root {
     --card-bg:      var(--vscode-editorWidget-background);
     --card-border:  var(--vscode-editorWidget-border, var(--vscode-contrastBorder));
     --card-focus:   var(--vscode-focusBorder);
     --text:         var(--vscode-foreground);
     --text-muted:   var(--vscode-descriptionForeground);
     --badge-bg:     var(--vscode-badge-background);
     --badge-fg:     var(--vscode-badge-foreground);
     --error:        var(--vscode-errorForeground);
     --font:         var(--vscode-font-family);
     --font-code:    var(--vscode-editor-font-family);
   }
   ```
2. **Icons from codicons,** VS Code's own icon font (`@vscode/codicons`: `symbol-function`, `symbol-class`, `folder`, `link`, `target`, `go-to-file`…).
   The sidebar already uses them through `ThemeIcon`; the same icons on the cards make the two views read as one product. Your CSP already allows fonts
   from the extension.
3. **VS Code's sizes.** 13 px UI text, 22 px rows, 4 px grid. No custom shadows or gradients.

Accessibility comes mostly for free if you keep doing what the code already does: real `<button>` elements with `aria-label` / `aria-expanded`.
Add a visible focus ring (`outline: 1px solid var(--card-focus)`) and check the canvas once with a high-contrast theme.

## 5. Code in a card: Shiki for reading, Monaco for editing

You just added Monaco (with Shiki for colours) for the read-only code view. It works, but look at what each one costs:

| | Monaco editor | Shiki `codeToHtml` |
|---|---|---|
| What it is | a full editor: cursor, selection model, scrolling, input | a function: text in, coloured HTML out |
| Per open card | a whole editor instance (memory, layout work on every pan/zoom) | a few DOM nodes |
| 30 cards open | will be felt **[measure]** | no problem |
| Needed for | **editing** (0.3) | **reading** (0.1, 0.2) |

Shiki is already in your bundle and already doing the colouring. For read-only cards, call it directly and render the HTML; keep line numbers with a CSS
counter. Then in 0.3, mount Monaco **only in the one card the user is editing** and swap back to static HTML when they leave it.

This is a suggestion to measure, not a demand: open 30 code cards on a real project, pan around, and look at the frame rate and memory in the webview
developer tools (*Developer: Open Webview Developer Tools*). If it is smooth, keep what you have until 0.3.

One more gap for later: the Shiki themes are fixed (`github-light` / `github-dark`), so the card's colours differ from the user's editor theme. Acceptable
for 0.x.

## 6. What is in 0.1, and what waits

| | 0.1 | 0.2 | 0.3 | later |
|---|---|---|---|---|
| Sidebar | Structure · Missing · attach from menus | | | search, CodeLens, hover |
| Canvas | tree layout · focus · breadcrumbs · minimap | call edges, graph layout, lazy expand | | groups, saved views |
| Card | link icon · doc count · add doc · code (read) · go to code | call markers, external leaves | **edit code** | zoom dialog, doc summary in the body |
| Docs | **doc panel** (rendered, read-only, live preview) · edit in a VS Code tab | | **edit in the panel** | freshness, links between docs |
| Feedback | Missing view · Problems lines | | | notifications, status bar |

Next: [`../08-decisions-to-log.md`](../08-decisions-to-log.md)
