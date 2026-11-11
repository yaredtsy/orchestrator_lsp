# 11. Git workflow, building for every platform, publishing

Written 2026-11-10 after reading `vnoc/` (`.github/workflows/ci.yml`, `scripts/package.mjs`, `extension/package.json`, `.gitignore`, the
branches) and `pylspt/`. Same goal as [`09-adding-a-language/06`](../09-adding-a-language/06-organizing-language-code.md): **grow fast, let
contributors join easily, keep releases boring.** I decided everything that is about structure; the few things that are yours to decide (licence,
names, what is public) are listed in §4 as questions.

```
git and release
├── README (this file)            where things stand · the picture · questions for you
├── 01-git-workflow.md            branches, pull requests, commit titles, versions, the release PR, hotfixes, the two repos
├── 02-building-every-platform.md the eight VS Code targets, one build script, the CI matrix, Linux / Windows / macOS details
├── 03-publishing.md              Marketplace + Open VSX, one-time setup, what the listing needs, rolling forward
├── 04-what-this-changes.md       the organization, revisited: toolchain, versions, pylspt, files to add
└── 05-language-packs.md          one download per language: each driver is its own small extension
```

**Revised 2026-11-10 (second pass)** after a review against the real repos. Three things in the first version would have failed
(release-please's tag starts no workflow; `--locked` against a stale lockfile; the hotfix tag shipped the old version number), and adding a language
touched six hand-edited places. The fixes are R10–R13; "download a language when needed" is R14–R16. Changed rows are marked.

## 1. Where things stand

| Thing | Today | Problem for publishing |
|---|---|---|
| CI | one job on `macos-latest`: typecheck, tests, build, `package.mjs`, upload the `.vsix` | builds **one** platform (darwin-arm64); never runs on Windows or Linux, where path and line-ending bugs live |
| `scripts/package.mjs` | accepts `--target`, but runs `cargo build --release` for the **host** only | a `--target win32-x64` on a Mac would package Mac binaries under a Windows label |
| `extension/package.json` | no `license`, `repository`, `icon` (PNG), `README`; the script passes `--skip-license --allow-missing-repository` | the Marketplace listing would be empty, and `vsce publish` warns or refuses |
| versions | extension `0.0.1`; crates `0.1.0`; no tags | nothing ties a bug report to a build |
| toolchains | vnoc root **1.94.1**, Python driver **1.98.0**, pylspt **1.98.0** | every platform build installs two toolchains and their targets |
| branches | `main`, `ui-rework` (pushed); no protection | a push to `main` can skip CI |
| repo hygiene | `*.vsix`, `target/`, `node_modules/` ignored ✓ · no `LICENSE` · no `.gitattributes` | Windows checkouts may turn test fixtures into CRLF and break byte-exact tests |

Sizes today (darwin-arm64, release): core 5.5 MB, Python driver 13.9 MB, `.vsix` 9.6 MB compressed.

## 2. The picture

```
contributor                   GitHub (vnoc)                                                users
────────────                  ──────────────────────────────────────────────               ─────
feat/sidebar-webview ──PR──►  ci.yml: only the jobs the changed paths need; one required check "ci-ok"
                              squash-merge → main (always releasable)
                                   │
                              release-please keeps one release PR open PER THING that changed:
                              "release vnoc 0.2.0" · "release driver-python 0.1.3"   (CHANGELOG from PR titles)
                                   │  you merge one
                              tag vnoc-v0.2.0 ──► release.yml          (the tag IS the version; no file is bumped)
                                              ├── build that one thing for its targets
                                              ├── smoke-test each binary where the runner can run it
                                              ├── GitHub Release with the .vsix files
                                              └── publish (waits for your one click) ──► VS Code Marketplace ─► VS Code
                                                                                     └► Open VSX ──────────► Cursor, VSCodium,
                                                                                                             Windsurf, Gitpod…
nightly.yml: everything × all targets, so a platform break is found on a normal day, not on release day.

The repos:
  vnoc             core · extension · webview · EVERY driver (drivers/<lang>/) · all plan docs (docs/)
  pylspt           the Python analyzer with the vendored ruff. Separate because each ruff update adds tens of MB of history.
  vnoc_playground  spikes only; an archive. No plan lives here once 09 and 11 are copied to vnoc/docs.
```

## 3. The decisions, in one line each

| # | Decision | File |
|---|---|---|
| **R1** | Trunk-based: `main` is protected and always releasable; short branches; squash merge; **the PR title** is a Conventional Commit (the only rule contributors must learn). | [`01`](01-git-workflow.md) |
| **R2** *(changed)* | **release-please** writes the CHANGELOG and the tag, using a GitHub App token so that its PR gets CI and its tag starts the build. It bumps no file (R10). | [`01`](01-git-workflow.md) §5 |
| **R3** *(changed)* | **One version** for the extension and the core (`vnoc-v0.2.0`). A bundled driver shares it; a language pack has its own (`driver-python-v0.1.3`). pylspt has its own. | [`01`](01-git-workflow.md) §4 |
| **R4** *(changed)* | **Eight targets**: `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `alpine-x64`, `alpine-arm64`, `win32-x64`, `win32-arm64`. Linux binaries are static musl, so the Alpine packages reuse them. **Three block a release** (`darwin-arm64`, `linux-x64`, `win32-x64`); the others publish when they are green. | [`02`](02-building-every-platform.md) |
| **R5** | Publish to **both** the VS Code Marketplace and Open VSX, from CI only, behind a GitHub environment that needs your approval. | [`03`](03-publishing.md) |
| **R6** | 0.x ships as normal releases with the **Preview** badge. The pre-release channel (odd/even minor versions) starts after 1.0. | [`03`](03-publishing.md) §4 |
| **R7** | **One Rust toolchain** for vnoc, its drivers and pylspt, bumped together. | [`04`](04-what-this-changes.md) §1 |
| **R8** *(replaced)* | A **nightly build in vnoc** of everything for all targets finds platform breaks, wherever the code lives. A split-out analyzer's own CI only needs Linux, macOS and Windows x64. | [`04`](04-what-this-changes.md) §3 |
| **R10** | **The tag is the version.** Crates and `package.json` say `0.0.0` in git; the build stamps the tag's number into the `.vsix` and the binaries. | [`01`](01-git-workflow.md) §4 |
| **R11** | **The `drivers/` folder is the list of languages.** Build and CI read `drivers/*/package.json`; no table of languages is kept by hand. `scripts/new-driver.mjs` scaffolds one. | [`02`](02-building-every-platform.md) §3 |
| **R12** | CI runs **only the jobs the changed paths need**; one job, `ci-ok`, is the single required check. | [`02`](02-building-every-platform.md) §4 |
| **R13** | The pylspt pin may be a `rev` on `main` between releases and must be a **tag when the Python driver is released**. pylspt is tagged by hand; no release-please there until someone outside uses it. | [`01`](01-git-workflow.md) §7 |
| **R14–R16** | **Language packs:** each driver ships as its own small extension from the same repo, installed when a file of that language is first opened; starts with the second language. | [`05`](05-language-packs.md) |
| **R17** | The plan docs live in **`vnoc/docs/` only**. | §2 |

## 4. Questions only you can answer

These are not structure; they are about who you publish as and what you give away. Each has my lean. Nothing is published until they are answered.

| # | Question | Why it matters | My lean |
|---|---|---|---|
| **Q-R1** ✅ | **Licence** for vnoc (and pylspt). | The Marketplace shows it; contributors need it before their first PR. pylspt vendors ruff (MIT), whose licence file must stay either way. | **Answered 2026-11-10: Apache-2.0.** The standard `LICENSE` text at the root of both repos; `"license": "Apache-2.0"` in every `package.json` and `Cargo.toml`. |
| **Q-R2** | **Publisher id** and extension name. The id is `publisher.name` (today `vnoc.vnoc`) and **can never change** once published. | Users' settings, keybindings and other extensions refer to it. | Check that `vnoc` is free on both the Marketplace and Open VSX first; if not, your GitHub name (`yaredtsy.vnoc`). |
| **Q-R3** | Are the **vnoc and pylspt repos public**? | Contributors need to clone both; the driver fetches pylspt by URL. A private pylspt needs a deploy key in vnoc's CI and shuts out contributors. Free GitHub Arm runners exist only for public repos (§02). | Both public at the first release. |

## 5. The order to build it in

The first version asked for all the process before anything had shipped. Do it in three steps:

| When | What |
|---|---|
| **before 0.1** | `LICENSE`, `.gitattributes`, one toolchain, the split build scripts with the version stamp, `drivers/python/package.json`, the three blocking targets, `ci.yml` with `ci-ok`, one dry-run release by hand (`workflow_dispatch`). Merge `ui-rework` in pieces, then turn on branch protection. |
| **at 0.1** | release-please with the App token, the `marketplace` environment, both stores, the five other targets (not blocking), `nightly.yml`. |
| **with the second language or the first outside contributor** | language packs ([`05`](05-language-packs.md)), `new-driver.mjs`, CODEOWNERS, the issue templates. |

Start with [`01-git-workflow.md`](01-git-workflow.md).
