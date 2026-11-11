# 11.04 What publishing changes in the organization

You asked whether building and publishing for every platform means rethinking how the code is organized. Mostly it confirms it: the split of
[`09-adding-a-language/06`](../09-adding-a-language/06-organizing-language-code.md) holds. Five things change, and one is a trigger to watch.

```
changes
├── 1. one Rust toolchain everywhere
├── 2. versions: no version in any file
├── 3. platform breaks are found by vnoc's nightly build
├── 4. the files to add, in both repos
├── 5. what stays the same, and why
└── 6. one download per language
```

Revised 2026-11-10 (second pass): §2, §3 and §6 changed; see the README of this folder for why.

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

## 2. Versions: no version in any file

The tag is the version (§01-4). Every `Cargo.toml` and `package.json` in vnoc says `0.0.0`; the build stamps the number from the tag. The
extension and the core release together as `vnoc-v…`; a language pack releases alone as `driver-<lang>-v…`
([`05`](05-language-packs.md)). pylspt and any other split-out analyzer keep their own version.

What this changes in `09-adding-a-language/06` §8: a new driver needs **no** version line anywhere. Its `Cargo.toml` says
`version.workspace = true` (which is `0.0.0`) or `version = "0.0.0"` when it has its own workspace.

## 3. Platform breaks are found by vnoc's nightly build, not by a rule for other repos

The first version of this plan gave every split-out analyzer a fifth condition: build for all eight targets in its own CI. The worry was real:
vnoc builds the analyzer from source on each release runner, so one that was only ever compiled on a Mac could break a release on Windows Arm.

The fix is simpler in one place than in every analyzer's repo: **`nightly.yml` in vnoc builds everything for every target, every day** (§02-4).
It catches a platform break whatever its cause (the core, a driver, an analyzer in another repo, a new runner image), and it is one matrix to
maintain instead of one per repo.

What a split-out analyzer still owes in its own CI: its tests pass on **Linux, macOS and Windows x64**. For pylspt that is the `syntax` and
`analysis` jobs of `09-adding-a-language/06` §7 on three runners.

## 4. The files to add

**vnoc**, at the repo root unless noted:

```
LICENSE                              Apache-2.0 (Q-R1)
CONTRIBUTING.md                      §01-8
.gitattributes                       §02-6
release-please-config.json           §01-5
.release-please-manifest.json
rust-toolchain.toml                  1.98.0 (changed)
Cargo.toml                           (changed) [workspace.package] version = "0.0.0", license = "Apache-2.0"
docs/                                (changed) + 09-adding-a-language/ and 11-git-and-release/, copied from the playground
.github/
├── workflows/
│   ├── ci.yml                       §02-4 (replaces today's single job)
│   ├── release.yml                  §02-4
│   ├── release-please.yml           §01-5 (with the GitHub App token)
│   └── nightly.yml                  §02-4
├── PULL_REQUEST_TEMPLATE.md
├── ISSUE_TEMPLATE/{bug.yml, language.yml}     later: with the first outside contributor
└── CODEOWNERS                                 later: same
scripts/
├── targets.mjs                      the platforms
├── list-drivers.mjs                 the languages, read from drivers/*/package.json
├── build-js.mjs · build-bin.mjs · pack.mjs
├── package.mjs                      (changed) all three for the host
├── new-driver.mjs                   later: with the second language
└── bump-pylspt.sh                   §01-7
extension/
├── README.md · CHANGELOG.md         §03-3
├── media/icon.png
└── package.json                     (changed) the listing fields; "preview": true; "version": "0.0.0"
drivers/python/
├── package.json                     NEW: "name", "private": true, the "vnoc" block. Becomes the language pack's manifest (05 §2)
├── Cargo.toml                       (changed) version = "0.0.0"
└── rust-toolchain.toml              1.98.0 (unchanged)
drivers/_template/                   later: with the second language
```

**pylspt**, on top of `09-adding-a-language/06` §4's layout:

```
LICENSE                              Apache-2.0 — vendor/ruff/LICENSE (MIT) stays as it is
.gitattributes                       §02-6
CHANGELOG.md                         by hand, one section per tag (no release-please yet: §01-7)
.github/workflows/ci.yml             syntax · analysis, on Linux, macOS and Windows x64 (§3 here)
CONTRIBUTING.md
```

## 5. What stays the same, and why

| Question | Answer |
|---|---|
| Does pylspt have to move into vnoc to build every platform? | **No.** Cargo fetches it by tag on each runner and the cache keeps it (§02-8). The reasons it is separate (its own toolchain pin, 42 MB of vendored ruff, useful without V-NOC) do not change. |
| Should pylspt publish binaries, and vnoc download them, instead of compiling it? | **No.** pylspt is a library, not a program; the driver that uses it lives in vnoc. Shipping prebuilt binaries across repos adds a second release pipeline and a second place where versions can disagree. Compile from source, cache well. |
| Can pylspt be published to crates.io? | **Not as it is**: crates.io requires every dependency to be on crates.io, and the vendored ty crates are not. A git dependency by tag works the same for vnoc, so nothing is lost. |
| Should every driver become its own extension? | **Yes, starting with the second language.** See §6 and [`05`](05-language-packs.md). |
| Does the driver stay in vnoc? | **Yes**, also when it ships as its own extension. It is the code that changes when the protocol does; one PR fixes the protocol and every driver. Separate download does not need a separate repo. |

## 6. One download per language

The first version of this plan waited for a trigger (a 50 MB package, or four bundled drivers) before splitting. You have since said that more
languages are coming and each should be downloaded only when needed, so the decision is made now and the trigger is gone:

**Each driver ships as its own small extension, a language pack, from this same repo.** Python stays bundled for 0.1; packs start in the release
that adds the second language, and Python moves out then. The whole design is in [`05-language-packs.md`](05-language-packs.md).

What 0.1 does so that nothing has to be moved later: `drivers/python/package.json` exists and the build reads it; tags are named `vnoc-v…`;
the build script already has a `binDir` per driver (§02-3).

## Draft rows for your log

| Draft row | Decision |
|---|---|
| **R1** | Trunk-based workflow: protected `main` (one required check, `ci-ok`), short branches, squash merge, Conventional-Commit PR titles. |
| **R2** *(changed)* | release-please writes the CHANGELOG and the tag, with a GitHub App token; it bumps no file. The tag starts `release.yml`. |
| **R3** *(changed)* | One version for the extension and the core (`vnoc-v…`); a bundled driver shares it, a language pack has its own (`driver-<lang>-v…`). Split-out analyzers have their own. |
| **R4** *(changed)* | Eight targets; Linux binaries static musl with mimalloc; Alpine packages reuse them. Three targets block a release, the others publish when green. A driver may list fewer targets. |
| **R5** | Publish to the VS Code Marketplace and Open VSX from CI only, behind an approval. |
| **R6** | 0.x releases carry `"preview": true`; the pre-release channel (odd/even minor) starts at 1.0. |
| **R7** | One Rust toolchain (1.98.0) in vnoc, its drivers and pylspt. |
| **R8** *(replaced)* | vnoc's nightly build covers every target for everything, including analyzers in other repos. Those repos test on Linux, macOS and Windows x64 only. |
| **R9** *(replaced by R14–R16)* | — |
| **R10** | The tag is the version: every `Cargo.toml` and `package.json` says `0.0.0`; the build stamps the tag's number. |
| **R11** | The `drivers/` folder is the list of languages: build and CI read `drivers/*/package.json`. |
| **R12** | CI runs only the jobs the changed paths need; `ci-ok` is the single required check. A nightly workflow builds all targets. |
| **R13** | The pylspt pin may be a `rev` on `main` and must be a tag at a release of the Python driver. pylspt is tagged by hand for now. |
| **R14–R16** | Language packs: see [`05`](05-language-packs.md). |
| **R17** | The plan docs live in `vnoc/docs/` only. |
| **Q-R1** ✅ | Licence: **Apache-2.0** for vnoc and pylspt (your decision, 2026-11-10). |

Back to: [`README.md`](README.md)
