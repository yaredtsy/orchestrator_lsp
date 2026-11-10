# 09. Adding a language

This folder is the guide you follow when you add a driver for a new language. [`06-drivers.md`](../06-drivers.md) explains the protocol and the driver
host; this folder answers "who does what, what must my driver do, and in what order do I build it".

```
adding a language
├── 01-who-does-what.md              extension · core · driver: the split, and what is still Python-only today
├── 02-what-a-driver-must-do.md      the contract, tier by tier (structure · markers · call trees + paging)
├── 03-files-and-driver-selection.md which files, which driver, files with no driver, missing drivers
├── 04-markers-and-unsafe-files.md   where an ID may be written, and the files it must never touch
├── 05-step-by-step.md               the checklist, with a decision sheet to fill in per language
└── 06-organizing-language-code.md   where a language's code lives: driver vs analyzer, the folder shape, pylspt's layout, releases
```

## One word first: "driver", not "LSP"

A V-NOC driver is easy to confuse with a language server (an "LSP"), because it borrows two things from the Language Server Protocol: the message
framing (`Content-Length: …`) and the `initialize` handshake. It is **not** a language server:

| | A language server (pyright, rust-analyzer…) | A V-NOC driver |
|---|---|---|
| Talks to | the editor | the V-NOC core |
| Methods | `textDocument/hover`, `completion`, … | `vnoc/outline`, `vnoc/planIds`, `vnoc/callTree`, … |
| Job | help you type | tell V-NOC what is in a file, and where an ID can go |

So nothing a driver does shows up in VS Code directly, and a driver does not replace or conflict with the user's Python or Rust extension.

## The goal this folder holds you to

**Adding a language = writing one new program and adding one line to the packaging script.** No change in the core, no change in the extension, no
change in the webview. That is not true today (the list of what is still Python-only is in `01`), and making it true is a one-time job that should be
done before the second driver, not during it.

## The short version

1. The **driver** knows the language. It reads text and answers questions. It never writes a file and never picks an ID.
2. The **core** knows V-NOC. It decides which files go to which driver, owns the index, the IDs and `.vnoc/`. It knows no language.
3. The **extension** knows VS Code. It starts the core, shows things, applies edits. It knows no language either.
4. A driver can ship useful at **tier 1** (structure only) and grow.
5. A file that no driver claims can still have docs, attached by path. V-NOC **never writes a marker into a file unless a driver planned that edit.**

Start with [`01-who-does-what.md`](01-who-does-what.md).
