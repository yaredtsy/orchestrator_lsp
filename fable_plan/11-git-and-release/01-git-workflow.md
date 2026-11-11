# 11.01 The git workflow

```
git workflow
├── 1. one rule: main is always releasable
├── 2. branches and pull requests
├── 3. the PR title is the commit message
├── 4. versions: the tag is the version
├── 5. a release, step by step (release-please)
├── 6. fixing an old release
├── 7. the two repos together
└── 8. what makes it easy for a newcomer
```

## 1. One rule: `main` is always releasable

Everything else follows from it. **Trunk-based development**: there is one long-lived branch, `main`; work happens on short branches that live
days, not weeks; anything merged into `main` could be released that afternoon.

Why this and not *git-flow* (a `develop` branch, release branches, merges in both directions): git-flow suits teams that ship on a fixed calendar
and support several versions at once. V-NOC ships when something is ready, supports one version at a time, and has one maintainer and a growing set
of contributors. Every extra long-lived branch is a place where work waits and conflicts grow.

Unfinished work that must merge anyway (a half-built sidebar) hides behind a setting, not a branch:

```jsonc
// extension/package.json — "contributes.configuration"
"vnoc.experimental.webviewSidebar": { "type": "boolean", "default": false, "description": "Use the new sidebar (unfinished)." }
```

Remove the setting when the feature is done. Two releases later at most; a stale flag is a bug.

**Branch protection on `main`** (GitHub → Settings → Rules → New ruleset, target `main`):

- require a pull request (no direct pushes, also not for you: the CI check is the point)
- require **one** status check: `ci-ok` (§02-4). Not the individual jobs: a job that is skipped because its paths did not change never reports,
  and a required check that never reports leaves the PR waiting forever
- required approvals: **0** while you are the only maintainer (GitHub does not let you approve your own PR)
- require a linear history (only squash merges)
- block force pushes and deletion

Turn this on **after** `ui-rework` is merged. That branch is already weeks long, which is what this rule exists to prevent; merge it in pieces
(§2) first.

## 2. Branches and pull requests

```
<type>/<short-name>          feat/sidebar-webview · fix/windows-paths · docs/release-plan · driver-python/header-comment
```

- One branch = one pull request = one change a reviewer can hold in their head. The `ui-rework` branch you have now is fine as a name; the habit
  to build is to **merge it in pieces** (the seven steps of `10-ui-rework/05-build-order.md` are seven pull requests, each leaving `main` working).
- Open a **draft** PR as soon as the branch exists. CI runs on every push, and others see what is in flight.
- Rebase on `main` when it moves (`git pull --rebase origin main`); do not merge `main` into the branch. The squash merge makes the branch's own
  history disappear anyway, so its commits can be messy.
- After merge the branch is deleted (Settings → "Automatically delete head branches").
- Docs change in the **same** PR as the code they describe.

**Merge method: squash only** (Settings → Pull Requests: untick "merge commits" and "rebase merging"). Each PR becomes exactly one commit on
`main`, whose message is the PR title. `git log --oneline` on `main` then reads as a list of changes, `git revert <one hash>` undoes one whole
change, and `git bisect` finds the PR that broke something.

## 3. The PR title is the commit message

Because of the squash merge, contributors need to learn **one** rule, for **one** line: the PR title follows
[Conventional Commits](https://www.conventionalcommits.org/).

```
<type>(<scope>): <what changed, in the imperative>

feat(webview): sidebar as a webview view
fix(core): time out a driver that stops answering
feat(protocol)!: kind becomes an open string            ← "!" = breaking
docs: git and release plan
```

| type | goes in the CHANGELOG as | bumps (while 0.x) |
|---|---|---|
| `feat` | Features | the minor: 0.2.0 → 0.3.0 |
| `fix`, `perf` | Bug fixes / Performance | the patch: 0.2.0 → 0.2.1 |
| `feat!` / `fix!` (breaking) | ⚠ Breaking changes | the minor while 0.x; the major after 1.0 |
| `docs`, `test`, `refactor`, `build`, `ci`, `chore` | not listed | nothing |

Scopes, one per top-level folder: `core` · `domain` · `protocol` · `driver-python` (and `driver-<lang>`) · `extension` · `webview` · `shared` ·
`docs` · `release`.

A check enforces it, so nobody has to remember: a small workflow runs `amannn/action-semantic-pull-request` on every PR and fails with a readable
message when the title is wrong. Individual commits on the branch can say anything. Check the **type** only, not the scope: a new language then
needs no change to the check, and release-please's own PR title (`chore(main): release vnoc 0.3.0`) passes.

The scope is for people reading the log. Which release a change belongs to is decided by the **files it touched**, not by the scope (§5).

> **Why it matters:** the version number and the CHANGELOG are then computed from `main`'s history (§5). Nobody edits a version by hand, and
> nobody writes release notes from memory.

## 4. Versions: the tag is the version

**No file in git holds the version.** Every `Cargo.toml` and `package.json` says `0.0.0` and stays that way. A release is a tag, and the build
stamps the tag's number into what it produces:

```
tag vnoc-v0.2.1
  └─ release.yml reads "0.2.1" from the tag name
       ├─ VNOC_VERSION=0.2.1 cargo build …        the binaries
       └─ pack.mjs writes "version": "0.2.1" into package.json IN THE CI CHECKOUT (never committed), then runs vsce
```

```rust
// crates/vnoc/src/main.rs (the same line in every driver)
const VERSION: &str = match option_env!("VNOC_VERSION") { Some(v) => v, None => "0.0.0-dev" };
```

> **🦀 Rust note: `option_env!`.** `env!("X")` reads an environment variable **while compiling** and fails the build if it is missing.
> `option_env!("X")` gives an `Option<&'static str>` instead. So a release build carries `0.2.1`, and a plain `cargo build` on your machine carries
> `0.0.0-dev`, which is the honest answer: it is not a release.

The core prints it (`vnoc --version`) and each driver sends it in `initialize`, so the Output panel and every bug report carry it.

Why not let release-please bump the files, as the first version of this plan said:

| With version files | With the tag |
|---|---|
| release-please edits `Cargo.toml` but not `Cargo.lock`; the release build uses `--locked` and **fails** on the stale lockfile | nothing to go stale |
| every driver with its own workspace is one more entry to keep in release-please's file list | a new driver needs no entry for its version |
| a hotfix tagged by hand (§6) builds with the **old** number from `package.json`, and the stores reject a version they already have | the hand-made tag is the number |

**What has a version:**

| Thing | Tag | When |
|---|---|---|
| the extension + the core (+ any driver bundled inside it) | `vnoc-v0.2.1` | from 0.1 |
| each language pack | `driver-python-v0.1.3` | from the release that starts language packs ([`05`](05-language-packs.md) §8) |
| pylspt (its own repo) | `v0.3.0` | its own pace |

A user reports "V-NOC 0.2.1 on Windows"; that names one build. The core and the drivers check each other by the **protocol number**, not by
version, so a language pack can release without the core and the other way round ([`05`](05-language-packs.md) §6).

> **Semantic versioning in one paragraph.** `MAJOR.MINOR.PATCH`. Before 1.0, the rule people follow is: `0.MINOR` changes may break things, `PATCH`
> never does. After 1.0, breaking = `MAJOR`. For vnoc "breaking" means **a committed file format changed** (`.vnoc/`, IDs, markers) or a setting was
> removed; that is rare by design (`02-mvp-and-releases.md` §1). For pylspt it means its public Rust API changed.

## 5. A release, step by step

[release-please](https://github.com/googleapis/release-please-action) does the bookkeeping. It is a GitHub Action that reads the PR titles merged
since the last tag and keeps **a pull request** open called *"chore(main): release vnoc 0.3.0"*. That PR contains the new CHANGELOG section and
nothing else of weight. It updates itself after every merge.

```
1. features and fixes merge into main, one PR each
2. release-please's PR grows: "vnoc 0.3.0 · Features: sidebar as a webview · Bug fixes: …"
3. you decide it is time: read the PR, merge it
4. release-please tags vnoc-v0.3.0 and creates a GitHub Release with that CHANGELOG section
5. the tag starts release.yml (§02): builds the .vsix files, attaches them to the Release
6. the publish job waits for your approval (§03), then uploads to both marketplaces
```

Configuration, two files at the repo root:

```jsonc
// release-please-config.json
{
  "bump-minor-pre-major": true,                             // 0.x: feat → minor, breaking → minor
  "include-component-in-tag": true,                         // tags are vnoc-v0.3.0, driver-python-v0.1.3
  "separate-pull-requests": true,                           // one release PR per thing, so each releases alone
  "packages": {
    ".": {
      "release-type": "simple",
      "component": "vnoc",
      "changelog-path": "extension/CHANGELOG.md"            // the Marketplace shows this file
      // when language packs start, add:  "exclude-paths": ["drivers"]
    }
    // when language packs start, one entry per language (scripts/new-driver.mjs writes it):
    // "drivers/python": { "release-type": "simple", "component": "driver-python" }
  }
}
```
```jsonc
// .release-please-manifest.json — the current version of each thing; release-please maintains it
{ ".": "0.1.0" }
```

**How it knows which release a change belongs to: the path.** A merged PR that touched files under `drivers/python/` goes into
`driver-python`'s next release; with `exclude-paths`, everything else goes into `vnoc`'s. A PR that touched both appears in both. So the folder
layout is also the release layout, and a new language is one entry here.

**[check]** when setting it up: the `simple` type also writes a small `version.txt` next to the CHANGELOG. It is harmless (nothing reads it); the
tag stays the source.

> **⚠ The token. This is the step that fails silently.** GitHub does not start workflows for things done with the default `GITHUB_TOKEN` (a guard
> against workflows that trigger themselves forever). If release-please runs with that token:
> - its release **PR gets no CI run**, so the required `ci-ok` check never reports and the PR cannot be merged;
> - its **tag starts no `release.yml`**, so step 5 never happens.
>
> Give release-please its own identity instead: a **GitHub App** installed on the repo (permissions: contents and pull requests, read + write),
> with `actions/create-github-app-token` making a short-lived token in the workflow. A fine-grained personal access token also works, but it
> expires and it is tied to your account; the App is not.
> ```yaml
> # .github/workflows/release-please.yml   (on: push to main)
> - uses: actions/create-github-app-token@v1
>   id: app
>   with: { app-id: ${{ vars.RELEASE_APP_ID }}, private-key: ${{ secrets.RELEASE_APP_KEY }} }
> - uses: googleapis/release-please-action@v4
>   with: { token: ${{ steps.app.outputs.token }} }
> ```

Why a release *PR* and not "tag whenever you like": a release is then a reviewed change like any other, its notes are visible before it ships, and
a contributor can see what is about to go out.

## 6. Fixing an old release

Usually there is nothing to do: `main` is releasable, so a fix merges into `main` and the next release (a patch, `0.3.1`) carries it.

Only when `main` already holds something that must not ship yet (and a setting cannot hide it):

```
git switch -c release/0.3 vnoc-v0.3.0     a branch from the tag
git cherry-pick <the fix's commit on main>
                                          add the "0.3.1" section to extension/CHANGELOG.md by hand, in the same PR
git push origin release/0.3               PR into release/0.3 → CI → merge
git tag vnoc-v0.3.1 && git push origin vnoc-v0.3.1
```

Because the tag is the version (§4), this hand-made tag builds and publishes `0.3.1` with no file to bump. A tag you push yourself starts
`release.yml` normally; the token problem of §5 is only about tags made by a workflow.

The fix always lands on `main` **first**, so it is never lost. `release/*` branches get the same protection rules as `main`. Expect this to be rare
before 1.0.

## 7. The two repos together

pylspt uses the same habits, with less machinery: protected `main`, squash merges, Conventional Commit PR titles (scopes `syntax`, `analysis`,
`dev`, `vendor`), tags `v0.x.y`. **No release-please there yet.** A pylspt release is `git tag v0.4.0 && git push origin v0.4.0` plus a hand-written
CHANGELOG section. Add release-please the day someone outside V-NOC depends on pylspt.

**The pin rule:**

| Where | The pylspt pin in `drivers/python/Cargo.toml` may be |
|---|---|
| on vnoc's `main`, between releases | a `rev` (a commit on pylspt's `main`) **or** a tag |
| at a release of the Python driver | a **tag** only. `release.yml` refuses to build a driver whose `Cargo.toml` has a `rev =` git dependency |
| anywhere in git | never a local path |

Why not "always a tag", as the first version said: call trees (0.2) will need many small changes on both sides. A tag per change means a pylspt
release, then a bump PR, for every one of them. A `rev` is just as reproducible (Cargo locks the exact commit either way); the tag matters for
what users get, so it is required exactly there.

A change that needs both (a field the driver needs from pylspt), as in `09-adding-a-language/06` §8:

```
1. pylspt     PR → merge
2. vnoc       PR "build(driver-python): pylspt <rev>" made by scripts/bump-pylspt.sh <rev or tag>
              (edits the pin in drivers/python/Cargo.toml, runs cargo update -p pylspt-syntax, runs the driver's tests)
3. before releasing the Python driver:  tag pylspt, then  scripts/bump-pylspt.sh v0.4.0
```

During development a gitignored `[patch]` points the driver at your local pylspt (06-drivers §7).

**Why pylspt is its own repo at all, said once:** not the toolchain (that is unified, [`04`](04-what-this-changes.md) §1). It is the vendored ruff.
pylspt's `.git` is 232 MB today against vnoc's 4 MB, and every ruff update adds tens of MB that can never be removed from history. Kept apart, a
person who clones vnoc to fix the canvas never downloads it.

## 8. What makes it easy for a newcomer

Files at the root of vnoc that a first-time contributor reads or meets. `README`, `CONTRIBUTING` and the PR template come before 0.1; the issue
templates and CODEOWNERS can wait for the first outside contributor.

| File | What it says |
|---|---|
| `README.md` | what V-NOC is (one screenshot), install, **build from source in three commands**, where the docs are |
| `CONTRIBUTING.md` | setup (rustup, pnpm), the test loop, the PR title rule (§3), how to run the extension (F5), how to add a language (link to `09-adding-a-language/`) |
| `.github/PULL_REQUEST_TEMPLATE.md` | what changed · how it was tested · screenshot for UI changes · "docs updated?" |
| `.github/ISSUE_TEMPLATE/bug.yml` | V-NOC version, OS, VS Code version, the V-NOC **Output** panel log, steps |
| `.github/ISSUE_TEMPLATE/language.yml` | "support for <language>": asks the decision-sheet questions of `09-adding-a-language/05` §1 up front |
| `.github/CODEOWNERS` | who reviews what: `/drivers/python/ @yaredtsy` · later `/drivers/go/ @<the Go maintainer>`. A new language can bring its own owner, which is how this scales past one person |
| labels | `area/core` · `area/ui` · `lang/python` · `good first issue` · `help wanted` |

The three-command build is the one to protect. Today it is: `pnpm install`, `cargo build`, `cargo build` inside `drivers/python`, then F5. Add
`pnpm dev:setup` that runs the Rust builds too, so it is `pnpm install && pnpm dev:setup`, then F5.

Next: [`02-building-every-platform.md`](02-building-every-platform.md)
