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
└── 04-what-this-changes.md       the organization, revisited: toolchain, versions, pylspt, files to add
```

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
feat/sidebar-webview ──PR──►  ci.yml: Linux · macOS · Windows, every PR
                              squash-merge → main (always releasable)
                                   │
                              release-please keeps a "release 0.2.0" PR open,
                              with the version bump and CHANGELOG from PR titles
                                   │  you merge it
                              tag v0.2.0 ──► release.yml
                                              ├── build × 8 targets (vnoc core + driver per platform; JS once)
                                              ├── smoke-test each binary where the runner can run it
                                              ├── GitHub Release with the 8 .vsix files
                                              └── publish (waits for your one click) ──► VS Code Marketplace ─► VS Code
                                                                                     └► Open VSX ──────────► Cursor, VSCodium,
                                                                                                             Windsurf, Gitpod…
pylspt: the same workflow, smaller. Its tag (v0.3.0) is pinned in vnoc/drivers/python; a bump is one PR.
```

## 3. The decisions, in one line each

| # | Decision | File |
|---|---|---|
| **R1** | Trunk-based: `main` is protected and always releasable; short branches; squash merge; **the PR title** is a Conventional Commit (the only rule contributors must learn). | [`01`](01-git-workflow.md) |
| **R2** | **release-please** writes the version bump and the CHANGELOG; merging its PR tags the release; the tag starts the build. | [`01`](01-git-workflow.md) §5 |
| **R3** | **One version** for everything in vnoc (extension, core, driver): `vnoc 0.2.0` means one build of all three. pylspt has its own. | [`01`](01-git-workflow.md) §4 |
| **R4** | **Eight targets** from the first release: `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `alpine-x64`, `alpine-arm64`, `win32-x64`, `win32-arm64`. Linux binaries are static musl, so the Alpine packages reuse them. | [`02`](02-building-every-platform.md) |
| **R5** | Publish to **both** the VS Code Marketplace and Open VSX, from CI only, behind a GitHub environment that needs your approval. | [`03`](03-publishing.md) |
| **R6** | 0.x ships as normal releases with the **Preview** badge. The pre-release channel (odd/even minor versions) starts after 1.0. | [`03`](03-publishing.md) §4 |
| **R7** | **One Rust toolchain** for vnoc, its drivers and pylspt, bumped together. | [`04`](04-what-this-changes.md) §1 |
| **R8** | A split-out analyzer must **build for all eight targets in its own CI**. It becomes the fifth condition of the "own repo" rule. | [`04`](04-what-this-changes.md) §3 |

## 4. Questions only you can answer

These are not structure; they are about who you publish as and what you give away. Each has my lean. Nothing is published until they are answered.

| # | Question | Why it matters | My lean |
|---|---|---|---|
| **Q-R1** | **Licence** for vnoc (and pylspt). | The Marketplace shows it; contributors need it before their first PR; without one, nobody may legally use or contribute. pylspt vendors ruff (MIT), whose licence file must stay either way. | MIT or Apache-2.0 for both (Apache adds a patent grant). Pick before the repo goes public. |
| **Q-R2** | **Publisher id** and extension name. The id is `publisher.name` (today `vnoc.vnoc`) and **can never change** once published. | Users' settings, keybindings and other extensions refer to it. | Check that `vnoc` is free on both the Marketplace and Open VSX first; if not, your GitHub name (`yaredtsy.vnoc`). |
| **Q-R3** | Are the **vnoc and pylspt repos public**? | Contributors need to clone both; the driver fetches pylspt by URL. A private pylspt needs a deploy key in vnoc's CI and shuts out contributors. Free GitHub Arm runners exist only for public repos (§02). | Both public at the first release. |

Start with [`01-git-workflow.md`](01-git-workflow.md).
