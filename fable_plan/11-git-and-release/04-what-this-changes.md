# 11.04 What publishing changes in the organization

You asked whether building and publishing for every platform means rethinking how the code is organized. Mostly it confirms it: the split of
[`09-adding-a-language/06`](../09-adding-a-language/06-organizing-language-code.md) holds. Five things change, and one is a trigger to watch.

```
changes
├── 1. one Rust toolchain everywhere
├── 2. one version inside vnoc
├── 3. an analyzer in its own repo must build for every target (a fifth condition)
├── 4. the files to add, in both repos
├── 5. what stays the same, and why
└── 6. the trigger to watch: the size of the package
```

## 1. One Rust toolchain everywhere

Today: vnoc's root `rust-toolchain.toml` says **1.94.1**; `drivers/python/rust-toolchain.toml` and pylspt say **1.98.0**. A release then installs
two toolchains on each of six runners, and adds every target to both.

**Decision: one version, 1.98.0, in all three files**, bumped together in one PR per repo. The driver's own workspace stays (its `Cargo.lock` must
follow pylspt's exact `salsa` and ruff versions, which is a lockfile reason, not a toolchain one); only the toolchain is unified.

> **🦀 Rust note: what `rust-toolchain.toml` does.** When you run `cargo` in a folder, `rustup` looks for this file upwards from that folder and uses
> the version it names, installing it if needed. Two files with two versions mean two complete compilers on disk and in CI caches, and a target added
> to one (`rustup target add aarch64-unknown-linux-musl`) is missing in the other. One version removes that class of "works in this folder, fails in
> that one" problems.

Rule for the future: the toolchain follows pylspt (it needs the newest, because ruff's code does). Bump pylspt first, then vnoc in the same day.

## 2. One version inside vnoc

The extension, the core and the drivers inside vnoc share **one version** (§01-4), written by release-please into the three files. pylspt and any
other split-out analyzer keep their own version and are pinned by tag.

What this changes in `09-adding-a-language/06` §8: nothing for pylspt; and a new driver *inside* vnoc gets no version of its own (its `Cargo.toml`
says `version.workspace = true` when it joins the root workspace, or is added to release-please's `extra-files` when it has its own).

## 3. A fifth condition for an analyzer in its own repo

`09-adding-a-language/06` §2 says when an analyzer moves to its own repo. Publishing adds one obligation for every analyzer that has moved:

> **It must build, and pass its tests, for all eight targets in its own CI.**

The reason: vnoc builds the driver, and therefore the analyzer, from source on each release runner. An analyzer that has only ever been compiled on
a Mac breaks vnoc's release on Windows Arm, and the failure shows up in the wrong repo, on release day.

For pylspt that means a CI job like vnoc's `bin` matrix (§02-4), building `pylspt-syntax` and `pylspt-analysis` for the six Rust targets, and
running the tests on the three runnable ones (Linux, macOS, Windows x64). The vendored ruff already builds on all of them upstream; what this
catches is your own code (paths, `cfg` mistakes) and the trimmed vendor closure.

## 4. The files to add

**vnoc**, at the repo root unless noted:

```
LICENSE                              Q-R1
CONTRIBUTING.md                      §01-8
.gitattributes                       §02-6
release-please-config.json           §01-5
.release-please-manifest.json
rust-toolchain.toml                  1.98.0 (changed)
.github/
├── workflows/
│   ├── ci.yml                       §02-4 (replaces today's single job)
│   └── release.yml                  §02-4
├── PULL_REQUEST_TEMPLATE.md
├── ISSUE_TEMPLATE/{bug.yml, language.yml}
└── CODEOWNERS
scripts/
├── targets.mjs                      the target table
├── build-js.mjs · build-bin.mjs · pack.mjs
├── package.mjs                      (changed) all three for the host
└── bump-pylspt.sh                   §01-7
extension/
├── README.md · CHANGELOG.md         §03-3
├── media/icon.png
└── package.json                     (changed) the listing fields; "preview": true
drivers/python/
├── rust-toolchain.toml              1.98.0 (unchanged)
└── vnoc-driver-python.json          the manifest (S14), copied into bin/ by build-bin
```

**pylspt**, on top of `09-adding-a-language/06` §4's layout:

```
LICENSE                              Q-R1 (yours) — vendor/ruff/LICENSE stays as it is
.gitattributes                       §02-6
release-please-config.json           "release-type": "rust", one package per crate or one for the workspace
.github/workflows/ci.yml             syntax · analysis · the six-target build (§3 here)
CONTRIBUTING.md
```

## 5. What stays the same, and why

| Question | Answer |
|---|---|
| Does pylspt have to move into vnoc to build every platform? | **No.** Cargo fetches it by tag on each runner and the cache keeps it (§02-8). The reasons it is separate (its own toolchain pin, 42 MB of vendored ruff, useful without V-NOC) do not change. |
| Should pylspt publish binaries, and vnoc download them, instead of compiling it? | **No.** pylspt is a library, not a program; the driver that uses it lives in vnoc. Shipping prebuilt binaries across repos adds a second release pipeline and a second place where versions can disagree. Compile from source, cache well. |
| Can pylspt be published to crates.io? | **Not as it is**: crates.io requires every dependency to be on crates.io, and the vendored ty crates are not. A git dependency by tag works the same for vnoc, so nothing is lost. |
| Should every driver become its own extension? | **Not now.** See §6. |
| Does the driver stay in vnoc? | **Yes.** It is the code that changes when the protocol does, and the release builds it next to the core. |

## 6. The trigger to watch: the size of the package

Every bundled driver adds to every user's download, whether or not they use that language. Today a `.vsix` is 9.6 MB. When the Python driver uses
only `pylspt-syntax`, it shrinks **[measure]**; when call trees add `pylspt-analysis` in 0.2, it grows again.

**Re-open this decision when either is true:** a platform's `.vsix` passes **50 MB**, or there are **four** bundled drivers. The options then,
roughly from least to most work:

1. strip and size-optimize the drivers (`opt-level = "s"` for the largest crates, `lto = "fat"` is already on)
2. download a driver on first use of its language (the plan_vs VS-D30 idea, with its signed manifest and checksums), keeping Python bundled
3. one small extension per language (`vnoc-python`, `vnoc-go`) that hands its driver to the main extension through an exported API

Not before the trigger: each option adds moving parts, and until then the bundled-everything package is the simplest thing for users and
contributors.

## Draft rows for your log

| Draft row | Decision |
|---|---|
| **R1** | Trunk-based workflow: protected `main`, short branches, squash merge, Conventional-Commit PR titles. |
| **R2** | release-please makes the version bump and CHANGELOG; its merged PR tags the release; the tag starts `release.yml`. |
| **R3** | One version for the extension, core and bundled drivers. Split-out analyzers have their own and are pinned by tag. |
| **R4** | Eight targets per release; Linux binaries static musl with mimalloc; Alpine packages reuse them. |
| **R5** | Publish to the VS Code Marketplace and Open VSX from CI only, behind an approval. |
| **R6** | 0.x releases carry `"preview": true`; the pre-release channel (odd/even minor) starts at 1.0. |
| **R7** | One Rust toolchain (1.98.0) in vnoc, its drivers and pylspt. |
| **R8** | An analyzer in its own repo must build for all eight targets in its own CI (fifth condition of `09-adding-a-language/06` §2). |
| **R9** | Re-open bundling when a `.vsix` passes 50 MB or there are four bundled drivers. |

Back to: [`README.md`](README.md)
