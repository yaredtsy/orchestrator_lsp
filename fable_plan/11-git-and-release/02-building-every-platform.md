# 11.02 Building for every platform

```
building
├── 1. why one .vsix per platform
├── 2. the eight targets
├── 3. one build script, two halves
├── 4. the three workflows: ci.yml (every PR), release.yml (every release tag), nightly.yml
├── 5. Linux: static musl binaries
├── 6. Windows: paths, line endings, .exe
├── 7. macOS: two architectures from one runner
├── 8. speed: caching the slow part
└── 9. signing, and what users' machines will say
```

## 1. Why one `.vsix` per platform

The extension itself is JavaScript and runs everywhere. The core and the drivers are native programs: a macOS Arm binary does not run on Windows.
VS Code solves this with **platform-specific extensions**: you upload one `.vsix` per platform under the same version, and each user's VS Code
downloads the one for its machine.

> **🧩 VS Code note: `vsce package --target`.** The `--target` flag writes the platform into the package's manifest. The Marketplace then serves
> `vnoc-0.2.0@win32-x64.vsix` to a Windows x64 user and `…@darwin-arm64.vsix` to an Apple Silicon Mac. A user on a platform you did not upload sees
> "not available for your platform" instead of a broken install. rust-analyzer ships exactly this way.

The alternative, one `.vsix` holding every platform's binaries, would be about 8 × 20 MB and every user would download all of it. No.

## 2. The eight targets

| VS Code target | Rust target triple | Built on | Can the runner run it (smoke test)? |
|---|---|---|---|
| `darwin-arm64` | `aarch64-apple-darwin` | `macos-latest` (Arm) | ✅ |
| `darwin-x64` | `x86_64-apple-darwin` | `macos-latest`, cross-compiled | ✅ through Rosetta **[check that the runner has Rosetta]** |
| `linux-x64` | `x86_64-unknown-linux-musl` | `ubuntu-latest` | ✅ |
| `linux-arm64` | `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm` | ✅ |
| `alpine-x64` | — reuses `linux-x64`'s binaries | — | — |
| `alpine-arm64` | — reuses `linux-arm64`'s binaries | — | — |
| `win32-x64` | `x86_64-pc-windows-msvc` | `windows-latest` | ✅ |
| `win32-arm64` | `aarch64-pc-windows-msvc` | `windows-11-arm` | ✅ |

Not built: `linux-armhf` (32-bit Raspberry Pi; ty is heavy for it), `web` (vscode.dev: no native programs can run there).

**Runner availability [check before the first release]:** the Arm runners for Linux (`ubuntu-24.04-arm`) and Windows (`windows-11-arm`) are free
for **public** repositories. For a private repo, cross-compile instead: `linux-arm64` from `ubuntu-latest` with `cargo zigbuild`, `win32-arm64` from
`windows-latest` with the MSVC Arm64 tools. Those builds then get no smoke test. This is one reason for Q-R3 (public repos).

How much to promise users, said plainly in the README:

| Support level | Targets | Meaning |
|---|---|---|
| **tested** | `darwin-arm64` | you use it daily |
| **checked** | `linux-x64`, `win32-x64` | CI runs the full test suite, including the extension tests in a real VS Code window |
| **built** | the other five | CI builds them and checks the binaries start; nobody uses them daily yet. Bug reports welcome. |

**Only the first two rows block a release.** If `win32-arm64` fails to build on release day, the release still goes out for the targets that
are green, and the missing one is published later under the same version (§4). A version can receive more platform packages after it was first
published; it can never receive the same platform twice.

A language may also support fewer targets than the core: `"targets": [...]` in its `package.json` (§3). A new driver can start on three platforms
and grow.

## 3. One build script, two halves

`scripts/package.mjs` today builds everything for the host. Split it, because the JavaScript is the same on every platform and only the binaries
differ:

```
scripts/
├── targets.mjs             the platforms (a table)
├── list-drivers.mjs        the languages (read from the drivers/ folders)
├── build-js.mjs            webview (vite) + extension (esbuild) → extension/media/webview, extension/dist      ONCE per release
├── build-bin.mjs           the core and/or drivers for one VS Code target → a bin/ folder                    ONCE per target
├── pack.mjs                stamp the version, then vsce package --target <target>                           ONCE per target
├── new-driver.mjs <lang>   copy drivers/_template/ to drivers/<lang>/
└── package.mjs             build-js + build-bin + pack for the host (what you run locally, as today)
```

Two small files hold everything the scripts and CI need to know. The **targets** are a table. The **languages** are not a table at all: they are
the folders under `drivers/`.

```js
// scripts/targets.mjs
export const TARGETS = {
  "darwin-arm64": { rust: "aarch64-apple-darwin",        os: "macos-latest",     required: true },
  "darwin-x64":   { rust: "x86_64-apple-darwin",         os: "macos-latest" },
  "linux-x64":    { rust: "x86_64-unknown-linux-musl",   os: "ubuntu-latest",    required: true },
  "linux-arm64":  { rust: "aarch64-unknown-linux-musl",  os: "ubuntu-24.04-arm" },
  "alpine-x64":   { reuse: "linux-x64" },
  "alpine-arm64": { reuse: "linux-arm64" },
  "win32-x64":    { rust: "x86_64-pc-windows-msvc",  exe: ".exe", os: "windows-latest", required: true },
  "win32-arm64":  { rust: "aarch64-pc-windows-msvc", exe: ".exe", os: "windows-11-arm" },
};
```

```js
// scripts/list-drivers.mjs — the ONE place that knows which languages exist. It reads the folders; nobody edits a list.
import { existsSync, readdirSync, readFileSync } from "node:fs";

export function listDrivers() {
  return readdirSync("drivers", { withFileTypes: true })
    .filter((d) => d.isDirectory() && existsSync(`drivers/${d.name}/package.json`))
    .map((d) => {
      const pkg = JSON.parse(readFileSync(`drivers/${d.name}/package.json`, "utf8"));
      return {
        lang: d.name,                                                    // "python"
        dir: `drivers/${d.name}`,
        program: `vnoc-driver-${d.name}`,                                // the naming rule; no field needed
        workspace: existsSync(`drivers/${d.name}/Cargo.lock`) ? `drivers/${d.name}` : ".",   // own lockfile = own workspace
        targets: pkg.vnoc?.targets ?? "all",
      };
    });
}
// `node scripts/list-drivers.mjs --json` prints the list; ci.yml and release.yml turn it into their job matrix (§4).
```

`drivers/<lang>/package.json` is the file that later becomes the language pack's manifest ([`05`](05-language-packs.md) §2). For 0.1 it needs
only `"name"`, `"private": true` and the `vnoc` block.

```js
// scripts/build-bin.mjs <what> <target> [--version 0.3.0]      what = "vnoc" | "driver-python" | "all"
const t = TARGETS[target];
const env = { ...process.env, VNOC_VERSION: version ?? "0.0.0-dev" };            // 01 §4: the tag is the version
for (const p of programsFor(what)) {                                              // the core, and/or drivers from listDrivers()
  run("rustup", ["target", "add", t.rust], p.workspace);                        // per toolchain; a no-op when present
  run("cargo", ["build", "--release", "--locked", "-p", p.program, "--target", t.rust], p.workspace, env);
  copy(`${p.workspace}/target/${t.rust}/release/${p.program}${t.exe ?? ""}`, p.binDir);
}
```

- `--locked` makes the build fail if `Cargo.lock` is out of date, instead of quietly resolving new versions on a release machine. It is safe now
  because no release step edits a `Cargo.toml` (01 §4).
- `p.binDir` is `extension/bin/` while a driver is bundled, and `drivers/<lang>/bin/` once it is a language pack. That one line is the whole
  difference between the two for the build.
- The driver's manifest (`vnoc-driver-<lang>.json`, S14) is written into the same `bin/` by the driver's own build, as
  `09-adding-a-language/03` §1 says.

**Adding a language changes none of these scripts.** `node scripts/new-driver.mjs go` copies `drivers/_template/` to `drivers/go/` and renames
what is inside; from then on the build and CI see it.

## 4. The three workflows

### `ci.yml`: every push and pull request

```yaml
jobs:
  changes:         # seconds. Decides what the other jobs do.
    runs-on: ubuntu-latest
    outputs: { rust: …, js: …, drivers: … }      # drivers = JSON list for the matrix below
    steps: dorny/paths-filter (which top-level folders changed) · node scripts/list-drivers.mjs --json --changed
           # a driver counts as changed when drivers/<lang>/** changed, OR crates/vnoc-driver-protocol/** OR scripts/** did

  checks:          # fast, Linux: what most PRs need to know in a few minutes
    needs: changes
    if: needs.changes.outputs.rust == 'true' || needs.changes.outputs.js == 'true'
    steps: pnpm install --frozen-lockfile · pnpm typecheck · pnpm -r test ·
           cargo fmt --check · cargo clippy --workspace -- -D warnings · cargo test --workspace

  driver:          # one job per CHANGED driver and OS. Python (ty) is slow; a webview PR never runs it.
    needs: changes
    if: needs.changes.outputs.drivers != '[]'
    strategy:
      matrix:
        driver: ${{ fromJSON(needs.changes.outputs.drivers) }}
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps: cargo test -p vnoc-driver-${{ matrix.driver.lang }} (in matrix.driver.workspace) · the conformance test

  extension:       # the real VS Code window, on the three "checked" platforms
    needs: changes
    strategy: { matrix: { os: [ubuntu-latest, macos-latest, windows-latest] } }
    steps: build-bin all for the host · build-js · extension test suite (on Linux under xvfb-run: VS Code needs a display) ·
           pack and upload the host's .vsix as an artifact (what early testers install between releases, §03-4)

  pr-title:        # §01-3
    if: github.event_name == 'pull_request'
    uses: amannn/action-semantic-pull-request

  ci-ok:           # THE ONLY REQUIRED CHECK (01 §1). Always runs; green when nothing it waited for failed.
    needs: [checks, driver, extension, pr-title]
    if: always()
    steps: fail if any needed job's result is "failure" or "cancelled" ("skipped" is fine)
```

Today's single macOS job becomes these.

> **⚙ GitHub Actions note: why one `ci-ok` job, and not "require every job".** A job skipped by a path filter never reports a result. Branch
> protection that requires it then waits forever, and a docs-only PR can never be merged. (Workflow-level `paths-ignore`, which the first version
> of this plan suggested, has exactly this problem.) So the filters live **inside** the workflow, and one job that always runs sums the others up.
> `fromJSON` turns a JSON string made by an earlier job into the matrix of a later one; that is how the list of drivers comes from the folders
> and not from this file.

### `release.yml`: every release tag

```yaml
on:
  push: { tags: ["vnoc-v*", "driver-*-v*"] }
  workflow_dispatch: { inputs: { tag: { required: true } } }    # re-run for a tag: a late target, or the dry run before 0.1

jobs:
  plan:                                  # seconds
    outputs: { what: …, version: …, required: …, extra: … }
    steps: split the tag: "driver-python-v0.1.3" → what=driver-python, version=0.1.3
           the targets for it, from targets.mjs and (for a driver) its package.json, as two lists: required and extra
           refuse a driver whose Cargo.toml has a `rev =` git dependency (01 §7)

  js:                                    # once; only when what == vnoc
    steps: build-js · upload artifact "js"

  bin-required:                          # darwin-arm64 · linux-x64 · win32-x64
    needs: [plan, js]
    strategy: { fail-fast: false, matrix: { include: ${{ fromJSON(needs.plan.outputs.required) }} } }
    runs-on: ${{ matrix.os }}
    steps: &build
           build-bin <what> ${{ matrix.target }} --version <version> · smoke test · pack ·
           (linux only) pack alpine-<arch> from the same bin/ · upload the .vsix files

  bin-extra:                             # the other targets: same steps, but a failure does not stop the release
    needs: [plan, js]
    continue-on-error: true
    strategy: { fail-fast: false, matrix: { include: ${{ fromJSON(needs.plan.outputs.extra) }} } }
    steps: *build

  github-release:                        # attaches every .vsix that exists to the Release
    needs: [bin-required, bin-extra]

  publish:                               # 03-publishing.md
    needs: github-release
    environment: marketplace             # waits for your approval
```

- One workflow releases the core **or** one language pack; the tag says which. A Go fix builds one small program for its targets and nothing else.
- **A target that failed** is fixed on `main`, then published late: start `release.yml` by hand (`workflow_dispatch`) with the same tag. The
  publish step skips what the stores already have (§03-5), so only the missing platform goes up.
- While Python is bundled (0.1), `what=vnoc` builds the core and every driver into `extension/bin/`.

### `nightly.yml`: once a day

```yaml
on: { schedule: [{ cron: "0 3 * * *" }], workflow_dispatch: {} }
jobs:
  all-targets:      # the bin jobs above for vnoc AND every driver, all targets, with the smoke test. Publishes nothing.
  report:           # on failure: open (or update) one issue titled "nightly build is failing", with the target and the log link
```

This is what makes "three blocking targets" safe: the other five are still built every day, so a break is found the morning after the PR that
caused it. It also covers code that lives in another repo (pylspt): vnoc builds it from source here, on every target, so that repo does not need
this matrix itself.

**The smoke test**, per target where the runner can run the binary: `vnoc --version` prints the release's version, and a three-message script
(initialize → `vnoc/outline` on a one-line Python file → shutdown) gets an answer from the driver. It catches the classic platform breakages (a
missing system library, a wrong architecture, a crash at start) in seconds.

## 5. Linux: static musl binaries

A Rust program built normally on Linux links against the machine's C library (`glibc`) and then refuses to start on a machine with an **older**
glibc ("version `GLIBC_2.39` not found"). Users run every Linux distribution there is, and containers and remote dev boxes are often old.

**Build against musl and link statically.** The binary then carries its own C library and runs on any Linux of the same architecture, Alpine
included. That is why the two `alpine-*` packages need no build of their own.

```
sudo apt-get install musl-tools                                # on the x64 and Arm Ubuntu runners
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

> **🦀 Rust note: musl's allocator, and `#[global_allocator]`.** musl's own `malloc` is much slower than glibc's for programs that allocate a lot,
> and a parser and type checker allocate a lot. ruff ships its musl builds with a faster allocator for this reason. Do the same in the core and the
> driver:
> ```rust
> // drivers/python/src/main.rs (and crates/vnoc/src/main.rs)
> #[cfg(target_env = "musl")]
> #[global_allocator]
> static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
> ```
> `#[global_allocator]` tells Rust which allocator every `Box`, `Vec` and `String` in the program uses. The `cfg` limits it to musl builds, so the
> macOS and Windows builds keep the system allocator. **[measure]** the driver's outline time on a large file with and without it.

## 6. Windows: paths, line endings, `.exe`

Three things break on Windows that never break on a Mac:

1. **Paths.** `\` instead of `/`, drive letters, case-insensitive names. The extension already uses `path.sep` in places; the core stores
   repo-relative paths with `/` (`RepoPath`). The `extension` CI job on `windows-latest` is what proves it. Expect the first run to find something.
2. **Line endings.** Git on Windows may check text files out with CRLF (`core.autocrlf`). Then a test fixture is no longer the same bytes, and a
   byte-exact test (UTF-16 positions, round trips, snapshots) fails on Windows only. Fix it for everyone, once, with a `.gitattributes` at the root of
   **both** repos:
   ```gitattributes
   * text=auto eol=lf
   # fixtures are byte-exact test input: never convert them, whatever the platform
   **/fixtures/**   -text
   **/conformance/** -text
   *.snap           -text
   ```
   And add CRLF fixtures **on purpose** (`fixtures/crlf.py`): Windows users have CRLF files, and the driver must keep them CRLF when it inserts an ID.
3. **`.exe`.** The extension looks for `bin/vnoc` (`extension.ts`, `binary()`): it already adds `.exe` on `win32`. The build table above copies
   `vnoc.exe`. The driver path the extension passes to the core needs the same suffix (it does today).

## 7. macOS: two architectures from one runner

`macos-latest` is an Arm machine. It builds Intel binaries with `--target x86_64-apple-darwin`; Apple's SDK on the runner contains both. So both
`darwin-*` packages come from the same runner type, and nothing depends on GitHub keeping Intel Mac runners.

**Minimum macOS version.** Set `MACOSX_DEPLOYMENT_TARGET=11.0` in the build environment. Without it the binary is marked for the runner's macOS
version and refuses to start on older ones. 11.0 is the first macOS that runs on Apple Silicon and is old enough for nearly everyone.

## 8. Speed: caching the slow part

ty and ruff are most of the build time. A cold release build of the Python driver takes minutes per target **[measure]**; most CI runs should not
pay that.

- **`Swatinem/rust-cache`** in every Rust job, pointed at the workspace that job builds (`.` for the core and most drivers,
  `matrix.driver.workspace` for a driver with its own lockfile). No list of drivers here either:
  ```yaml
  - uses: Swatinem/rust-cache@v2
    with:
      workspaces: ${{ matrix.driver.workspace || '.' }}
      key: ${{ matrix.target }}        # one cache per target: they do not share compiled code
  ```
  It caches `~/.cargo/registry`, `~/.cargo/git` (so pylspt's repository is not downloaded again each run) and the dependency part of `target/`.
- **The split of `pylspt-syntax`** (`09-adding-a-language/06` §4) is the biggest speed-up: the 0.1 driver does not compile ty at all.
- Releases are rare; a 15-minute release build is fine. PR CI is what must stay fast, and it builds only the host target.

## 9. Signing, and what users' machines will say

| Layer | Signed by | Action needed |
|---|---|---|
| the `.vsix` | the Marketplace signs every extension on upload; VS Code checks it on install | none |
| macOS Arm binaries | the Rust toolchain's linker signs them ad hoc (required for Arm Macs to run anything) | none |
| binaries inside the extension | not signed | none expected **[check]**: VS Code downloads and unpacks extensions itself, so the files do not get the "downloaded from the internet" mark that wakes Gatekeeper (macOS) or SmartScreen (Windows). rust-analyzer and others ship unsigned binaries this way. Confirm once on a clean Mac and a clean Windows machine with an installed `.vsix` (open question Q6 in `08-decisions-to-log.md`). |

If the check fails on either machine: Apple Developer ID signing plus notarization (99 USD a year) for macOS, an Authenticode certificate for
Windows. Both fit into the `bin` job as one step after the build; decide only if the check shows you need them.

Next: [`03-publishing.md`](03-publishing.md)
