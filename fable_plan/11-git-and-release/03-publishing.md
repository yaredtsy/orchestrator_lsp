# 11.03 Publishing

```
publishing
├── 1. two stores: the VS Code Marketplace and Open VSX
├── 2. one-time setup
├── 3. what the listing needs (package.json, README, icon, CHANGELOG)
├── 4. versions on the stores: Preview now, pre-releases after 1.0
├── 5. the publish job
├── 6. when a release is bad: roll forward
└── 7. the first release, as a checklist
```

## 1. Two stores

| Store | Who installs from it | Account |
|---|---|---|
| **VS Code Marketplace** (Microsoft) | VS Code | a *publisher* in the Marketplace management page, tied to a Microsoft / Azure DevOps account |
| **Open VSX** (Eclipse Foundation) | Cursor, VSCodium, Windsurf, Gitpod, Eclipse Theia, code-server | an *Eclipse* account, then a *namespace* on open-vsx.org |

The Marketplace's terms allow only Microsoft's own products to install from it, so every VS Code fork uses Open VSX. Many of the developers most
likely to try a tool like V-NOC use Cursor. **Publish to both, every release, from the same `.vsix` files.**

## 2. One-time setup

```
1. Q-R2 answered: the publisher id (= Open VSX namespace) is chosen and FREE on both stores.
2. Marketplace: create the publisher (marketplace.visualstudio.com/manage).
   Create a Personal Access Token in Azure DevOps: Organization "All accessible organizations", scope Marketplace → Manage.
3. Open VSX: sign in with GitHub, sign the publisher agreement, create the namespace:  npx ovsx create-namespace <id> -p <token>
   Then ask for it to be verified (an issue on the open-vsx repo); a verified namespace shows a check mark and stops others from claiming the name.
4. GitHub → vnoc → Settings → Environments → New "marketplace":
     required reviewer: you          (the publish job waits for your click)
     deployment tags: vnoc-v*, driver-*-v*    (nothing else may publish)
     secrets: VSCE_PAT, OVSX_PAT
5. Put a reminder in your calendar: Azure DevOps tokens expire (one year at most). An expired token fails the publish job, not the build.
```

The tokens live **only** in that environment. A pull request, even from a contributor's fork, never sees them.

## 3. What the listing needs

The Marketplace page is built from the package. Today's `package.json` would produce an empty page, and the packaging script hides the warnings
(`--skip-license --allow-missing-repository`). Remove those two flags and add:

```jsonc
// extension/package.json
{
  "name": "vnoc",
  "displayName": "V-NOC",
  "publisher": "<Q-R2>",
  "description": "See your code as a graph and keep docs attached to functions and classes, even after a rename.",
  "version": "0.0.0",                                         // stays 0.0.0 in git; pack.mjs stamps the tag's number (01 §4)
  "preview": true,                                           // §4: the "Preview" badge while 0.x
  "license": "Apache-2.0",                                   // Q-R1, answered 2026-11-10
  "icon": "media/icon.png",                                  // 128×128 PNG. The Marketplace does not accept SVG here.
  "repository": { "type": "git", "url": "https://github.com/yaredtsy/vnoc" },
  "bugs": { "url": "https://github.com/yaredtsy/vnoc/issues" },
  "homepage": "https://github.com/yaredtsy/vnoc#readme",
  "categories": ["Visualization", "Other"],
  "keywords": ["documentation", "graph", "call graph", "canvas", "python"],
  "galleryBanner": { "color": "#1e1e1e", "theme": "dark" }
}
```

Files the package must carry (add them to `extension/.vscodeignore`'s allow-list, which today lets through only `package.json`, `dist/`, `media/`
and `bin/`):

| File | Shown as | Content |
|---|---|---|
| `extension/README.md` | the listing page | what it does in two sentences · a GIF of attach-a-doc-then-rename · the four steps to start · supported languages and platforms (§02-2's table) · known limits · link to issues |
| `extension/CHANGELOG.md` | the "Changelog" tab | written by release-please (§01-5) |
| `extension/LICENSE` | the "License" link | a copy of the root `LICENSE` (the build script copies it, so there is one source) |
| `extension/media/icon.png` | the icon | 128 × 128, readable at 32 × 32, works on light and dark backgrounds |

The README is the store page most people decide on. Write it last before the first release, from a fresh machine's point of view.

**Apache-2.0 in practice** (three small things, once):

- `LICENSE` at the root of vnoc and of pylspt: the standard Apache-2.0 text, unchanged. No per-file headers are required; do not add them.
- `license = "Apache-2.0"` in `[workspace.package]` of the root `Cargo.toml` (crates inherit it with `license.workspace = true`) and in every
  `package.json`, language packs included.
- pylspt keeps `vendor/ruff/LICENSE` (MIT) beside the vendored code. Because the driver's binary contains that code, the packages that carry
  the Python driver also ship a `THIRD-PARTY-NOTICES` file with ruff's licence text. `cargo about` can generate it for all Rust dependencies
  **[check when setting up `pack.mjs`]**.

A **language pack** needs the same four files in its own folder (`drivers/<lang>/`), and the same fields; see [`05`](05-language-packs.md) §2.

## 4. Versions on the stores

The stores accept only `MAJOR.MINOR.PATCH`: no `0.2.0-beta.1`. And a version, once uploaded, can never be uploaded again.

**Until 1.0: normal releases with `"preview": true`.** The Preview badge tells people it is early. Every release goes to everyone. Early testers who
want builds between releases download the `.vsix` from the GitHub Actions run of any `main` commit (the `ci.yml` artifact) and install it with
*Extensions: Install from VSIX…*.

**From 1.0: the pre-release channel.** VS Code lets users opt into pre-release versions of an extension. The stores need the two lines to have
different version numbers, and the convention (Microsoft's own) is:

```
release:      1.EVEN.patch     1.0.0, 1.0.1, 1.2.0 …          vsce publish
pre-release:  1.ODD.patch      1.1.0, 1.1.1, 1.3.0 …          vsce publish --pre-release
```

Starting that before 1.0 would make `0.1`, `0.2`, `0.3` (the feature releases of `02-mvp-and-releases.md`) mean two things at once. So it waits.

## 5. The publish job

The last job of `release.yml` (§02-4). It runs after the GitHub Release has all eight files, and waits for your approval in the `marketplace`
environment.

```yaml
publish:
  needs: github-release
  runs-on: ubuntu-latest
  environment: marketplace
  steps:
    - download every .vsix artifact into dist/
    - run: npx @vscode/vsce publish --skip-duplicate --packagePath dist/*.vsix
      env: { VSCE_PAT: ${{ secrets.VSCE_PAT }} }
    - run: for f in dist/*.vsix; do npx ovsx publish --skip-duplicate "$f" -p "$OVSX_PAT"; done
      env: { OVSX_PAT: ${{ secrets.OVSX_PAT }} }
```

- `vsce publish --packagePath` with several files uploads all the platform packages of one version.
- `--skip-duplicate` makes the job safe to run twice for one tag: what a store already has is skipped. That is how a target that failed on
  release day is added later (§02-4), and how a half-finished publish is finished.
- The same job publishes the core or a language pack; it uploads whatever `.vsix` files the build made for that tag.
- The two stores are separate steps: if Open VSX is down, the Marketplace release still went out, and you re-run only the failed step (GitHub's
  "Re-run failed jobs").
- Nobody publishes from a laptop. If it is not in CI, it did not happen: every published file then has a public build log and a tag.

## 6. When a release is bad: roll forward

You cannot replace a published version, and unpublishing is drastic (the Marketplace can remove the whole extension, not one version, and
users' installs then break). So the plan for a bad release is always the same:

```
1. fix on main (or on release/0.x, §01-6)
2. release 0.x.(y+1) as usual
3. users get it within hours: VS Code checks for extension updates by itself
```

If the bug can lose users' docs, say so at the top of the CHANGELOG entry of the fix, and pin an issue on GitHub until the fix is out.

## 7. The first release, as a checklist

```
□ Q-R1 licence: Apache-2.0 LICENSE at the root of vnoc and pylspt; THIRD-PARTY-NOTICES in the package (§3)
□ Q-R2 publisher id: free on both stores, for `vnoc` and the `vnoc-*` pack names; set in package.json
□ Q-R3 repos public
□ .gitattributes in both repos (§02-6)
□ one toolchain (04-what-this-changes.md §1)
□ every Cargo.toml and package.json at version 0.0.0; VNOC_VERSION read by the core and the driver (§01-4)
□ drivers/python/package.json; build scripts split; ci.yml with ci-ok; release.yml (§02-3, §02-4)
□ ui-rework merged; branch protection on, requiring only ci-ok (§01-1)
□ package.json fields, README, icon, CHANGELOG (§3)
□ one dry run: start release.yml by hand for a test tag with the publish job not approved; install each .vsix on a real machine for
  darwin-arm64, linux-x64, win32-x64
□ the pylspt pin is a tag, not a rev (§01-7)
□ the GitHub App for release-please; release-please set up; first manifest version 0.1.0 (§01-5)
□ nightly.yml on
□ Gatekeeper / SmartScreen check on a clean Mac and a clean Windows machine (§02-9)
□ environment "marketplace" with the two tokens and you as reviewer
□ merge the release PR → approve the publish → install from the store on a machine that never had V-NOC
```

Next: [`04-what-this-changes.md`](04-what-this-changes.md)
