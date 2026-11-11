# 11.05 Language packs: one download per language

You said you may add JavaScript / TypeScript, Rust and other languages, and that each should be **downloaded only when needed**. This file is the
plan for that: how a language is stored in git, how it is released, and how it reaches a user. It replaces the "wait for 50 MB" trigger that
[`04`](04-what-this-changes.md) §6 had.

```
language packs
├── 1. the decision: a language is its own small extension
├── 2. what a language pack is (one package.json, no code)
├── 3. how V-NOC finds the drivers that are installed
├── 4. how a user gets a language
├── 5. in git: one folder per language, one tag per release
├── 6. versions: what keeps an old pack and a new core working together
├── 7. add, update, release, remove
├── 8. when to build this
└── 9. to check before building
```

## 1. The decision

**Each language's driver ships as its own VS Code extension, called a language pack: `vnoc-python`, `vnoc-typescript`, `vnoc-rust`.** The main
extension `vnoc` holds the core, the UI and no driver. All of them live in the one `vnoc` git repo.

The other way to do "download when needed" is to write a downloader: the extension fetches a driver from GitHub Releases on first use, checks a
checksum and a signature, and stores it. I did not choose it, because the stores already do every part of that job:

| Job | Own downloader | Language pack on the stores |
|---|---|---|
| serve the right binary for the platform | you write it | the store does it (`--target`, as in [`02`](02-building-every-platform.md) §1) |
| verify the download | you design checksums, signatures, key storage | the Marketplace signs every package; VS Code checks it |
| updates | you write a version check | VS Code updates extensions by itself |
| company proxies, offline machines | the download fails behind many firewalls | whatever already works for their extensions works here; a `.vsix` can be copied by hand |
| Remote SSH / WSL / containers | you must download on the remote side, for its platform | VS Code installs the pack on the remote side by itself |
| remove a language | you write the cleanup | the user uninstalls the pack |
| a driver written by someone else | you design a registry | they publish an extension with one field (§3) |

The cost: one more listing per language on each store, and a release publishes more files. Both are handled by the same `release.yml`.

## 2. What a language pack is

A folder with a `package.json`, a README and the driver's binary. **No JavaScript.** VS Code allows an extension with no code (themes work this
way).

```jsonc
// drivers/go/package.json
{
  "name": "vnoc-go",
  "displayName": "V-NOC: Go",
  "publisher": "<Q-R2>",
  "version": "0.0.0",                              // stamped from the tag at release (01 §4)
  "license": "Apache-2.0",
  "engines": { "vscode": "^1.95.0" },
  "extensionKind": ["workspace"],                  // runs where the code is (matters for Remote SSH / WSL)
  "extensionDependencies": ["<Q-R2>.vnoc"],        // installing the pack installs V-NOC itself
  "vnoc": {
    "driver": "bin/vnoc-driver-go.json",           // the driver's manifest (S14), relative to this folder
    "languages": ["go"],                           // VS Code language ids: when to offer this pack (§4)
    "targets": "all"                               // or a list, e.g. ["darwin-arm64", "linux-x64", "win32-x64"]
  }
}
```

The `vnoc` block is read by three things, so nothing about a language is written twice:

| Reader | Uses | For |
|---|---|---|
| the main extension, at run time | `driver` | finding the installed driver (§3) |
| the main extension's build | `languages`, `name` | the table of packs it can offer (§4) |
| `scripts/list-drivers.mjs` (build and CI) | `targets`, the folder | what to build, and for which platforms ([`02`](02-building-every-platform.md) §3) |

> **🧩 VS Code note: `extensionDependencies`.** When a user installs `vnoc-go`, VS Code installs `vnoc` first if it is missing, and refuses to
> disable `vnoc` while `vnoc-go` is enabled. So a user can start from either extension and end up with a working pair.

## 3. How V-NOC finds the drivers that are installed

The main extension asks VS Code for every installed extension and keeps those that have the `vnoc.driver` field:

```ts
// extension/src/drivers.ts
import * as vscode from "vscode";
import * as path from "node:path";

/** The manifest file of every installed language pack. The core reads each one (S14). */
export function installedDriverManifests(): string[] {
  return vscode.extensions.all
    .filter((ext) => typeof ext.packageJSON?.vnoc?.driver === "string")
    .map((ext) => path.join(ext.extensionPath, ext.packageJSON.vnoc.driver));
}

// A pack was installed, updated or removed while VS Code is open: tell the core the new list.
vscode.extensions.onDidChange(() => sendDriversToCore(installedDriverManifests()));
```

The core already takes a **list** of drivers and routes files by their manifests
([`09-adding-a-language/03`](../09-adding-a-language/03-files-and-driver-selection.md) §1). The only change is where the list comes from: today the
extension's own `bin/`, then also every language pack's folder.

This is also the door for other people's drivers. Anyone can publish an extension with a `vnoc.driver` field, and V-NOC will use it. You do not
run a registry.

## 4. How a user gets a language

```
the user opens main.go
  └─ is a driver installed for language id "go"?
       ├─ yes → nothing to do
       └─ no  → is "go" in the table of known packs?
                 ├─ yes → one notification: "V-NOC has Go support. [Install] [Not now] [Never for Go]"
                 │          Install → vscode.commands.executeCommand("workbench.extensions.installExtension", "<Q-R2>.vnoc-go")
                 │          then onDidChange (§3) fires and the tree fills in, without a reload
                 └─ no  → the file shows as "no driver for this file" (S16); docs can still be attached by path
```

- The **table of known packs** (`language id → extension id`) is generated when the main extension is built, from every
  `drivers/*/package.json`. A new language is offered to users from the next `vnoc` release on; before that they can still find it in the store.
- "Never for Go" is remembered in VS Code's global state. Ask once per language, never again.
- The first-run case is the same flow: a new user opens a Python file and gets one click. The main extension's README lists every pack.

## 5. In git

Still one repo, one folder per language. The folder of
[`09-adding-a-language/06`](../09-adding-a-language/06-organizing-language-code.md) §3 gains the pack's files:

```
vnoc/drivers/<lang>/
├── package.json             NEW  the language pack (§2); also this driver's entry in the build
├── README.md                     doubles as the store page: what is supported, known gaps
├── CHANGELOG.md             NEW  written by release-please
├── DECISIONS.md
├── analysis/ · driver/           the two crates (members of the root Cargo workspace)
├── conformance/
└── bin/                          gitignored: build-bin.mjs puts the driver and its manifest here
```

**One tag per release of one thing:**

```
vnoc-v0.4.0             the main extension and the core
driver-python-v0.3.1    the Python pack
driver-go-v0.1.0        the Go pack
```

release-please knows which is which by **path**: a merged PR that changed files under `drivers/go/` goes into the Go pack's next release and its
CHANGELOG; everything outside `drivers/` belongs to `vnoc` ([`01`](01-git-workflow.md) §5). So a Go fix releases the Go pack alone: about six
builds of one small program, not the whole product.

## 6. Versions: what keeps an old pack and a new core working together

Once packs release on their own, a user can have `vnoc 0.6` with `vnoc-go 0.2`. Their versions say nothing about each other. What matters is the
**protocol number**, which already exists:

- the driver's manifest states the protocol it speaks; the core states the range it accepts (06-drivers §3);
- **additive** protocol changes (a new optional field, a new method behind a capability) do not change the number, so old packs keep working;
- a **breaking** change raises the number. The PR that does it fixes every driver in the repo, and the release order is: packs first, then `vnoc`.
  For one release the core accepts both the old and the new number, so no user is broken in the hours between the two updates;
- a pack the core cannot accept is not started, and the user sees one message: "V-NOC: Go needs an update" with an Update button.

That is the whole compatibility rule. Do not add version ranges between extensions on top of it.

## 7. Add, update, release, remove

| Job | Steps | Files touched outside `drivers/<lang>/` |
|---|---|---|
| **add a language** | `node scripts/new-driver.mjs go` copies the template folder and adds the release-please entry · write the driver · conformance test | 2 lines in `release-please-config.json` and its manifest, written by the script |
| **update a language** | a PR that changes its folder | 0 |
| **release a language** | merge that pack's release PR; approve the publish | 0 |
| **remove a language** | delete the folder and its release-please entry · mark the store listing deprecated (`vsce unpublish` is not needed) | the same 2 lines |
| **a language only on some platforms** | `"targets": [...]` in its `package.json` | 0 |

CI and the build find the folder by themselves ([`02`](02-building-every-platform.md) §3 and §4). A check in CI fails when a `drivers/*` folder has no
release-please entry, so the script's two lines cannot be forgotten.

What a user keeps when a pack is removed or uninstalled: all their `.vnoc/` data. The nodes show as "no driver for this file", and come back when
the pack does (S16).

## 8. When to build this

| Release | Shape |
|---|---|
| **0.1** | Python is **bundled** in the main `.vsix`, as today. Nothing in this file blocks 0.1. Two cheap things are done now so nothing has to move later: `drivers/python/package.json` exists (the build reads it), and tags are named `vnoc-v…`. |
| **the release that adds the second language** | §3 and §4 are built; Python leaves the main `.vsix` and becomes `vnoc-python`; the new language ships as a pack from its first day. |

Why not from 0.1: with one language, packs add a second listing and an install step and save nobody a download. Why not later than the second
language: every release that bundles two drivers makes every user download both, and moving a language out is easiest when there is one to move.

When Python moves out, existing users open a Python file and get the §4 notification once. No data changes.

## 9. To check before building

- **[check]** `workbench.extensions.installExtension` with an extension id works in Cursor and VSCodium (they install from Open VSX).
- **[check]** the driver keeps its executable bit after install on Linux and macOS (the bundled `bin/vnoc` relies on the same thing today).
- **[check]** a pack with no `main` appears in `vscode.extensions.all` right after install, without a window reload.
- **[check]** the publisher id (Q-R2) is free for `vnoc` **and** the `vnoc-*` names you expect, on both stores.
- A driver that is not a native program (a TypeScript driver that needs Node): its manifest's `program` would name a script, and the core would
  start it with VS Code's own Node. Decide that with the TypeScript driver's decision sheet, not here.

## Draft rows

| Draft row | Decision |
|---|---|
| **R14** | Each language's driver ships as its own extension (`vnoc-<lang>`, no code, `extensionDependencies` on `vnoc`), from the same repo. The main extension finds drivers through the `vnoc.driver` field of installed extensions and offers a known pack the first time a file of that language is opened. No own downloader. |
| **R15** | Packs start with the second language; Python is bundled until then and moves out in that release. |
| **R16** | A pack has its own version and tag (`driver-<lang>-v…`). Core and packs are kept compatible by the protocol number only. |

Back to: [`README.md`](README.md)
