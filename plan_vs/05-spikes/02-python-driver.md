# 05.02: S2 Python driver

**Goal:** `vnoc-driver-python` runs as its own process, spawned by the core from the global drivers folder, and serves outlines and lazy call trees from pylspt.
**Answers:** C1 (does `render_depth` bound the solving?), C2 (stdout hygiene), C4 (binary size), C11 (OS quarantine / Defender), C12 (lock crate, data dir),
C17 (per-function parse errors), C22 (which `pyproject.toml` sections count as dependencies).
**Time box:** 3 days. **Code:** `spikes/s2-python-driver/`. **Needs:** S1's `spike-core`.

## Build

1. **The adapter crate** `spike-driver-python`, depending on pylspt by `rev`, with a gitignored `.cargo/config.toml` `[patch]` pointing at `../pylspt` (VS-D65).
   pylspt itself gets **no** dependency on any V-NOC crate.
2. **Handshake:** the core sends `initialize { protocol: 1 }`; the driver answers `{ protocol: 1, name: "python", version }`. Wrong protocol → the core refuses and says why.
3. **`outline(file)`** → files, classes, functions with ranges. **`callTree(fn, depth: 3)`** and **`expand(callNode, depth: 3)`**. Both go through the adapter, which turns
   pylspt's types into protocol types.
4. **C1:** time `callTree` at depth 1, 3, 5 and "unbounded" on a real mid-size repo (e.g. `httpx` or `rich`). If depth 3 costs nearly as much as unbounded, solving isn't bounded
   and lazy loading needs work inside pylspt.
5. **C2:** run the driver with a stdout guard in the core: any byte that isn't a framed message fails the run. Include runs that trigger ruff/ty warnings and panics.
6. **C4:** release build with `strip = true`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`; measure per platform; zip it.
7. **Copy-on-first-run (VS-D30):** the core copies the bundled binary into `<data dir>/vnoc/drivers/python/<ver>/` under a lock file, then spawns it from there.
   **C11:** does macOS refuse to run the copy (quarantine attribute, Gatekeeper)? Does Windows Defender slow the first start? **C12:** `fd-lock` vs `fs4` on macOS,
   Linux, Windows and a network home directory; what `dirs::data_local_dir()` returns on Windows.
8. **C17:** a file with a syntax error in one function: can pylspt say *which* defs contain an error, so save-time IDs skip only those?
9. **C22:** list the `pyproject.toml` tables that hold dependencies across tools (`[project]`, `[dependency-groups]`, `[tool.poetry]`, `[tool.uv]`) and the lockfiles
   (`uv.lock`, `poetry.lock`, `pdm.lock`, `requirements*.txt`). This becomes the driver's "environment inputs" list (VS-D53).

> **🦀 Rust note: one process, two jobs, no shared stdout.**
> The driver's stdout is the protocol, so in the adapter:
> ```rust
> #![deny(clippy::print_stdout, clippy::print_stderr)]   // even eprintln! goes through tracing
> fn main() {
>     tracing_subscriber::fmt().with_writer(std::io::stderr).init();
>     let (conn, _io) = lsp_server::Connection::stdio();      // the only thing that writes stdout
>     // …
> }
> ```
> A vendored crate that calls `println!` would still break the stream. Step 5 is there to catch exactly that.

## Measure

- `outline` time per file and for the whole scope; `callTree` time at each depth (C1).
- Driver binary size, raw and zipped, per platform (C4).
- First-start time after copy, per OS (C11).

## Pass / fail

- ✅ The core spawns the driver from the data dir, the handshake works, and a depth-3 tree renders in the S1 canvas.
- ✅ Each of C1, C2, C4, C11, C12, C17, C22 has an answer with numbers below.

## Findings

*(empty until S2 runs)*

Next: [`03-edits-and-saving.md`](03-edits-and-saving.md)
