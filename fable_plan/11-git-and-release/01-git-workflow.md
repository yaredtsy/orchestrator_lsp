# 11.01 The git workflow

```
git workflow
├── 1. one rule: main is always releasable
├── 2. branches and pull requests
├── 3. the PR title is the commit message
├── 4. versions: one for vnoc, one for pylspt
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
- require the CI checks to pass (§02's `ci.yml` jobs)
- require a linear history (only squash merges)
- block force pushes and deletion

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
message when the title is wrong. Individual commits on the branch can say anything.

> **Why it matters:** the version number and the CHANGELOG are then computed from `main`'s history (§5). Nobody edits a version by hand, and
> nobody writes release notes from memory.

## 4. Versions: one for vnoc, one for pylspt

**vnoc has one version** for the extension, the core and every bundled driver. A user reports "V-NOC 0.2.1 on Windows"; that names one build of
all three. Inside, the core and the driver already check each other by the **protocol number**, so they never need separate versions while they ship
together.

Where the version is written (release-please updates all of them):

```
extension/package.json        "version": "0.2.1"            ← what the Marketplace shows
Cargo.toml                    [workspace.package] version   ← the core crates inherit it (version.workspace = true)
drivers/python/Cargo.toml     [package] version             ← its own workspace
```

The core prints it (`vnoc --version`) and the driver sends it in `initialize`, so the Output panel and every bug report carry it.

**pylspt has its own version** (`v0.3.0`), because it is a library with its own users and its own pace. vnoc pins a pylspt tag; a pylspt release
does not force a vnoc release.

> **Semantic versioning in one paragraph.** `MAJOR.MINOR.PATCH`. Before 1.0, the rule people follow is: `0.MINOR` changes may break things, `PATCH`
> never does. After 1.0, breaking = `MAJOR`. For vnoc "breaking" means **a committed file format changed** (`.vnoc/`, IDs, markers) or a setting was
> removed; that is rare by design (`02-mvp-and-releases.md` §1). For pylspt it means its public Rust API changed.

## 5. A release, step by step

[release-please](https://github.com/googleapis/release-please-action) does the bookkeeping. It is a GitHub Action that reads the PR titles merged
since the last tag and keeps **one pull request** open called *"chore(main): release 0.3.0"*. That PR contains the version bump in the three files
above and the new CHANGELOG section. It updates itself after every merge.

```
1. features and fixes merge into main, one PR each
2. release-please's PR grows: "0.3.0 · Features: sidebar as a webview · Bug fixes: …"
3. you decide it is time: read the PR, merge it
4. release-please tags v0.3.0 and creates a GitHub Release with that CHANGELOG section
5. the tag starts release.yml (§02): builds 8 .vsix files, attaches them to the Release
6. the publish job waits for your approval (§03), then uploads to both marketplaces
```

Configuration, two files at the repo root:

```jsonc
// release-please-config.json
{
  "packages": {
    ".": {
      "release-type": "simple",
      "package-name": "vnoc",
      "changelog-path": "extension/CHANGELOG.md",           // the Marketplace shows this file
      "bump-minor-pre-major": true,                         // 0.x: feat → minor, breaking → minor
      "extra-files": [
        { "type": "json", "path": "extension/package.json",    "jsonpath": "$.version" },
        { "type": "toml", "path": "Cargo.toml",                "jsonpath": "$.workspace.package.version" },
        { "type": "toml", "path": "drivers/python/Cargo.toml", "jsonpath": "$.package.version" }
      ]
    }
  }
}
```
```jsonc
// .release-please-manifest.json — the current version; release-please maintains it
{ ".": "0.1.0" }
```

**[check]** when setting it up: the `toml` updater changes `Cargo.toml`, not `Cargo.lock`. Add a step to `release.yml` that runs `cargo update -w`
(workspace crates only) before building, or the lockfile shows the old version. It does not change any dependency.

Why a release *PR* and not "tag whenever you like": a release is then a reviewed change like any other, its notes are visible before it ships, and
a contributor can see what is about to go out.

## 6. Fixing an old release

Usually there is nothing to do: `main` is releasable, so a fix merges into `main` and the next release (a patch, `0.3.1`) carries it.

Only when `main` already holds something that must not ship yet (and a setting cannot hide it):

```
git switch -c release/0.3 v0.3.0          a branch from the tag
git cherry-pick <the fix's commit on main>
git push origin release/0.3               PR into release/0.3 → CI → merge
git tag v0.3.1 && git push origin v0.3.1  the tag starts release.yml as usual
```

The fix always lands on `main` **first**, so it is never lost. `release/*` branches get the same protection rules as `main`. Expect this to be rare
before 1.0.

## 7. The two repos together

pylspt uses the same rules: protected `main`, squash merges, Conventional Commit PR titles (scopes `syntax`, `analysis`, `dev`, `vendor`), its own
release-please (`"release-type": "rust"`), tags `v0.x.y`.

A change that needs both (a field the driver needs from pylspt), as in `09-adding-a-language/06` §8:

```
1. pylspt     PR → merge → merge its release PR → tag v0.4.0
2. vnoc       PR "build(driver-python): pylspt v0.4.0" made by scripts/bump-pylspt.sh v0.4.0
              (edits the tag in drivers/python/Cargo.toml, runs cargo update -p pylspt-syntax, runs the driver's tests)
```

vnoc's `main` never points at an untagged pylspt commit, and never at a local path. During development a gitignored `[patch]` points the driver at
your local pylspt (06-drivers §7).

## 8. What makes it easy for a newcomer

Files at the root of vnoc that a first-time contributor reads or meets:

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
