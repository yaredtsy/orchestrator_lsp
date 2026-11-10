# 11.02 Building for every platform

```
building
├── 1. why one .vsix per platform
├── 2. the eight targets
├── 3. one build script, two halves
├── 4. the two workflows: ci.yml (every PR) and release.yml (every tag)
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

## 3. One build script, two halves

`scripts/package.mjs` today builds everything for the host. Split it, because the JavaScript is the same on every platform and only the binaries
differ:

```
scripts/
├── build-js.mjs            webview (vite) + extension (esbuild) → extension/media/webview, extension/dist      ONCE per release
├── build-bin.mjs <target>  core + every driver for one VS Code target → extension/bin/                       ONCE per target
├── pack.mjs <target>       vsce package --target <target>                                                   ONCE per target
└── package.mjs             all three for the host (what you run locally, as today)
```

The heart of `build-bin.mjs` is one table, so adding a target or a driver is one line:

```js
// scripts/targets.mjs
export const TARGETS = {
  "darwin-arm64": { rust: "aarch64-apple-darwin" },
  "darwin-x64":   { rust: "x86_64-apple-darwin" },
  "linux-x64":    { rust: "x86_64-unknown-linux-musl" },
  "linux-arm64":  { rust: "aarch64-unknown-linux-musl" },
  "alpine-x64":   { reuse: "linux-x64" },
  "alpine-arm64": { reuse: "linux-arm64" },
  "win32-x64":    { rust: "x86_64-pc-windows-msvc",  exe: ".exe" },
  "win32-arm64":  { rust: "aarch64-pc-windows-msvc", exe: ".exe" },
};

// every program that goes into bin/: where its Cargo workspace is, and its name
export const PROGRAMS = [
  { workspace: ".",              name: "vnoc" },
  { workspace: "drivers/python", name: "vnoc-driver-python", manifest: "vnoc-driver-python.json" },
];
```

```js
// scripts/build-bin.mjs (the core of it)
const t = TARGETS[target];
for (const p of PROGRAMS) {
  run("rustup", ["target", "add", t.rust], p.workspace);                       // per toolchain; a no-op when present
  run("cargo", ["build", "--release", "--locked", "--target", t.rust], p.workspace);
  copy(`${p.workspace}/target/${t.rust}/release/${p.name}${t.exe ?? ""}`, `extension/bin/`);
  if (p.manifest) copy(`${p.workspace}/${p.manifest}`, "extension/bin/");
}
```

`--locked` makes the build fail if `Cargo.lock` is out of date, instead of quietly resolving new versions on a release machine.

## 4. The two workflows

### `ci.yml`: every push and pull request

```yaml
jobs:
  checks:          # fast, Linux: what most PRs need to know in a few minutes
    runs-on: ubuntu-latest
    steps: pnpm install --frozen-lockfile · pnpm typecheck · pnpm -r test ·
           cargo fmt --check · cargo clippy --workspace -- -D warnings · cargo test --workspace

  driver-python:   # the slow one (ty); its own job so it runs in parallel and caches on its own
    strategy: { matrix: { os: [ubuntu-latest, macos-latest, windows-latest] } }
    steps: cargo test (in drivers/python) · the conformance test

  extension:       # the real VS Code window, on the three "checked" platforms
    strategy: { matrix: { os: [ubuntu-latest, macos-latest, windows-latest] } }
    steps: build-bin for the host · build-js · extension test suite (on Linux under xvfb-run: VS Code needs a display)

  pr-title:        # §01-3
    if: github.event_name == 'pull_request'
    uses: amannn/action-semantic-pull-request
```

Today's single macOS job becomes these. A docs-only PR can skip the Rust jobs with `paths-ignore: ["docs/**", "**/*.md"]`.

### `release.yml`: every tag `v*`

```yaml
on: { push: { tags: ["v*"] } }

jobs:
  js:                                    # once
    runs-on: ubuntu-latest
    steps: build-js · upload artifact "js"

  bin:                                   # once per target that has its own binaries
    needs: js
    strategy:
      fail-fast: false                   # one broken target must not hide the others' results
      matrix:
        include:
          - { target: darwin-arm64, os: macos-latest }
          - { target: darwin-x64,   os: macos-latest }
          - { target: linux-x64,    os: ubuntu-latest }
          - { target: linux-arm64,  os: ubuntu-24.04-arm }
          - { target: win32-x64,    os: windows-latest }
          - { target: win32-arm64,  os: windows-11-arm }
    steps: download "js" · build-bin ${{ matrix.target }} · smoke test · pack ${{ matrix.target }} ·
           (linux only) pack alpine-${{ arch }} from the same bin/ · upload the .vsix files

  github-release:                        # attaches all .vsix files to the Release release-please created
    needs: bin

  publish:                               # 03-publishing.md
    needs: github-release
    environment: marketplace             # waits for your approval
```

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

- **`Swatinem/rust-cache`** in every Rust job, with both workspaces listed:
  ```yaml
  - uses: Swatinem/rust-cache@v2
    with:
      workspaces: |
        .
        drivers/python
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
