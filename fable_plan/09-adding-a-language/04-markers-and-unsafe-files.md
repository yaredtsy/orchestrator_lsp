# 09.04 Markers, and the files a marker must never touch

Your worry is right: writing a comment into the wrong file breaks it. A comment in a JSON file is a syntax error; a new first line above a shebang
stops a script from running; a changed lock file fails its checksum. This file is the set of rules that makes that impossible by construction.

```
markers
├── 1. the one rule that protects every file
├── 2. choosing the marker for a definition (per language)
├── 3. choosing the marker for a file: placement, and when to refuse
├── 4. file types: can it carry a marker at all?
├── 5. what a refusal looks like to the user
└── 6. later: file markers for languages without a driver
```

## 1. The one rule

> **V-NOC writes into a source file only an edit that a driver planned for that file. No driver, no edit.**

Everything else follows from it:

- The **core** has no code that inserts text into source files. It only forwards a driver's `edits`. (Its own writes are `.vnoc/` and `.vnoc-id`, which
  are V-NOC's files.)
- A file with **no driver** is never touched. It can still have docs, found by path ([`03`](03-files-and-driver-selection.md) §4).
- A **driver** is the only place that knows the language's syntax, so it is the only place that can say "safe here" or "not safe at all". It can refuse
  a single definition (`skipped`), a whole file, or the whole capability (`ids.file: false`).
- A refusal is **never an error for the user's task.** The doc is attached anyway, by path and name, with `id_in_code = "no"`.

So the answer to "some things might not support comments" is: those things either have no driver, or have a driver that says no.

## 2. Choosing the marker for a definition

When you write a driver, you choose once where an ID lives on a function or class. The requirements:

| Requirement | Why |
|---|---|
| It **moves with the definition** under cut-and-paste and under the language's refactoring tools. | that is the marker's whole job |
| It is **inert**: no effect on behaviour, types, or what the compiler / linter reports with default settings. | V-NOC must not create warnings **[check per language]** |
| The language's standard **formatter keeps it** and does not move it away from the definition. | otherwise every format-on-save detaches docs |
| It can be found again by a **cheap parse** of one file, with no project and no type information. | the outline runs on every file at start |
| It is **one line**, and recognisable to a person: a teammate without V-NOC should guess what it is. | it shows up in code review |
| There is an obvious place for it even when the definition has **no doc comment yet**. | most functions have none |

Python's answer is the `ID:` line in the docstring (F1), with a comment form reserved for later (`[ids] style`).

For other languages nothing is decided. As a starting point for your decision sheet, the usual candidates:

| Language family | Candidate | Watch out for |
|---|---|---|
| has doc comments attached to a definition (Rust `///`, Go `//` above, Java/TS `/** */`, C# `///`) | a line inside or directly above the doc comment: `vnoc: <ID>` | doc generators print it; linters that require a doc comment *format*; Go's rule that a doc comment starts with the name |
| has attributes / decorators / annotations | an attribute carrying the ID | usually **not** inert (needs an import or a definition, changes the build). Avoid. |
| only line comments | a comment on the line above the definition: `// vnoc: <ID>` | formatters that separate a comment from what follows; blank-line rules |

The reserved text for the comment form is `vnoc: <ID>` (F1). Using the same text in every language means one search finds every marker in a repository.

**Whatever you choose is frozen for that language** the day a user commits it. A driver may later *read* a second form; it must never stop reading
the first.

## 3. Choosing the marker for a file

A file marker is a comment near the top (`# FileID: …` in Python). The driver decides the exact line. The rules below are generic; the list of
"things that must stay first" is per language.

### Placement

```
line 1   #!/usr/bin/env python3          ← must stay line 1          (shebang)
line 2   # -*- coding: utf-8 -*-         ← must stay in lines 1–2    (encoding declaration)
line 3   # FileID: 0Q8V5W4T2F7KX9M3B6D1RZ4H7K     ← the first line that is free
line 4   # Copyright 2026 …              (a licence block may also be kept above the marker: your choice, written down)
```

| Things that must stay first | Languages |
|---|---|
| shebang `#!` | every scripting language |
| encoding declaration | Python, Ruby (`# encoding:`), |
| `<?xml … ?>` / `<!DOCTYPE …>` | XML, HTML, SVG |
| `<?php` | PHP |
| `package …` / build constraints (`//go:build`) | Go: constraints must precede the package clause and be followed by a blank line |
| `"use strict"` / `"use client"` directives | JavaScript / TypeScript: a comment above them is fine, but keep them the first *statement* |
| front matter `---` … `---` | Markdown, some templating formats |
| a byte-order mark | any: keep it as the very first bytes |

### Preserve everything else

- Keep the file's **line endings** (`\r\n` stays `\r\n`): use the ending of the neighbouring line for the inserted one.
- Keep the **BOM**, the **final newline** (or its absence), and all indentation.
- Insert **one line**. Do not add a blank line unless the language's formatter would add it anyway.
- **Idempotent:** asked to write a file ID into a file that has one, answer with no edit (or a replacement of exactly that line when told to replace).

### When to refuse the whole file

Answer `skipped` (with the reason) instead of an edit when:

| Condition | How to notice | Reason shown |
|---|---|---|
| The file is **generated** | a header like `@generated`, `DO NOT EDIT`, `Code generated by … DO NOT EDIT.`, `autogenerated` in the first lines | "generated file" |
| The file does **not parse** well enough to find the top safely | your own parser | "syntax error at the top of the file" |
| The format has **no comments** | by file type (§4) | "this file type cannot hold a comment" |
| The file is **minified** or one enormous line | line length | "minified file" |
| A comment would **change meaning** here | per language (a template that prints comments, a file hashed by a tool) | say which |

A driver for a language where *no* file can safely take a comment simply answers `ids.file: false` in its capabilities, and the core never asks.

## 4. File types: can it carry a marker at all?

A reference for when you decide what a driver should claim. "No driver" in the last column means: leave these to docs-by-path.

| Kind of file | Comment syntax | Marker? |
|---|---|---|
| Python, Ruby, shell, R, Perl | `#` | yes, after shebang / encoding line |
| JS, TS, Java, C, C++, C#, Go, Rust, Swift, Kotlin, Dart, PHP | `//` or `/* */` | yes, respecting the "first things" table |
| SQL, Lua, Haskell | `--` | yes |
| HTML, XML, SVG, Vue templates | `<!-- -->` | possible, after the declaration; the comment is sent to browsers, so think twice |
| CSS, SCSS | `/* */` | yes |
| YAML, TOML, INI, Dockerfile, Makefile | `#` | syntactically yes; many are read by tools that are strict about the first line. Low value: prefer docs-by-path. |
| Markdown | `<!-- -->` (HTML comment) | possible but visible in some renderers; prefer docs-by-path |
| **JSON**, `package.json`, `tsconfig.json`\* | **none** | **never.** (\*`tsconfig` tolerates comments, but other tools reading it do not.) |
| **Lock files** (`Cargo.lock`, `package-lock.json`, `uv.lock`, `go.sum`) | — | **never**: rewritten by tools, sometimes checksummed |
| **Jupyter notebooks** `.ipynb` | JSON inside | **never** as a comment. A notebook driver would need a cell-metadata marker, which is its own design. |
| CSV, TSV, data files | none | never |
| Binary files, images, archives | — | never; not even read |
| Generated code, vendored code, build output | any | never (refuse, §3) |
| Files outside the project root, read-only files | any | never: the edit would fail or surprise |

The safe default for a new driver: claim only the extensions of real source files of your language, and leave configuration and data formats unclaimed.

## 5. What a refusal looks like to the user

Nothing breaks and nothing is silent.

```
Add doc on  config/settings.json
  → doc created, attached by path
  → the card / tree row shows the OUTLINED link icon
  → tooltip: "Attached by path. This file type cannot hold an ID, so a rename will need a Relink."
  → no "Write ID" action is offered for this node
```

```
Add doc on a function inside a file with a syntax error
  → doc created, attached by name
  → tooltip: "No ID written: syntax error inside this function. Use Write ID when it is fixed."
  → "Write ID" is offered
```

The difference between the two is whether a marker is *possible later*. Keep the driver's `reason` text short and human; the UI shows it as is.

In `node.toml` both are `id_in_code = "no"`. `"never"` stays what it is today: the **user's** choice for one node.

## 6. Later: file markers for languages without a driver

There is a cheap middle step between "no driver" and "a real driver": a tiny built-in driver that knows only comment syntax.

```
vnoc-driver-text   (idea, after 0.1)
├── tier 1:  no outline (a file is just a file)
├── ids.file: yes, for a fixed table of extensions → comment syntax   (.sql "--", .css "/* */", .sh "#", …)
└── the same refusal rules as §3 (shebang, generated, minified)
```

It would make file-level docs survive renames for many file types at almost no cost, and because it is a driver, rule §1 still holds. It should be
**opt-in per extension** in `vnoc.toml`, since the table in §4 shows how many of these files are risky. Not in 0.1.

Next: [`05-step-by-step.md`](05-step-by-step.md)
