# 05.01: S1 hello core

**Goal:** a VS Code command starts a Rust binary, asks it something over stdio, and shows the answer in a webview with one xyflow node.
**Answers:** B5 (LSP framing + `vscode-languageclient`), C3 (Rust transport crate), C6 (`#:schema` in TOML), C18 (watching above the opened folder).
**Time box:** 2 days. **Code:** `spikes/s1-hello-core/`.

## Build

1. **Generate an extension.** `npx --package yo --package generator-code -- yo code` → *New Extension (TypeScript)*, bundler **esbuild**. Press **F5**: a second
   VS Code window (the *Extension Development Host*) opens with your extension loaded. This loop (edit → F5) is how all extension work goes.
2. **A Rust binary `spike-core`** using the `lsp-server` crate. It answers `initialize` and one custom request `vnoc/ping` → `{ pong: true, pid }`. Logs go to **stderr**
   with `tracing`; nothing else may touch stdout (`#![deny(clippy::print_stdout)]`).
3. **Start it from the extension** with `vscode-languageclient`: `ServerOptions = { command: <path to spike-core>, transport: TransportKind.stdio }`, then
   `await client.start()` and `client.sendRequest("vnoc/ping")`. Show the reply in a notification first.
4. **A webview panel** (`vscode.window.createWebviewPanel`) loading a **Vite + React + @xyflow/react** bundle through `webview.asWebviewUri`, with a CSP and a nonce
   (02-03 "webview rules"). The webview posts `{type: "req", method: "vnoc/ping"}`; the host relays it to the core and posts the result back. Draw it as **one node**.
5. **Save and restore state:** `vscode.setState({…})` in the webview, `registerWebviewPanelSerializer` in the host. Close and reopen VS Code: the panel comes back.
6. **Workspace Trust:** declare `capabilities.untrustedWorkspaces = { supported: "limited", restrictedConfigurations: ["vnoc.server.path"] }`. Open the folder in
   Restricted Mode and check the extension still activates and `workspace.isTrusted` is `false`.
7. **Watch above the opened folder (C18):** open `repo/sub/` as the workspace, create a `RelativePattern(<repo root>, ".vnoc/**")` watcher, touch a file in `repo/.vnoc/`.
   Does the event arrive? Does it work in Restricted Mode?
8. **`#:schema` (C6):** a `vnoc.toml` with `#:schema ./schema.json` on line 1; check autocomplete with Even Better TOML and with Tombi.
9. **C3:** write the same ping server once with `async-lsp` (or `tower-lsp-server`) and compare: lines of code, how cancellation and notifications look, compile time.

> **🧩 VS Code note: why `vscode-languageclient` even though we're not a language server.**
> It's just a very good JSON-RPC-over-stdio client: it starts the process, restarts it if it crashes, frames messages (`Content-Length:` headers), handles
> request IDs and cancellation, and shows traffic in the Output panel when `vnoc.trace.server` is on. We use its custom-request API (`sendRequest("vnoc/…")`)
> and ignore the language features.

## Measure

- Time from activation to the first `pong` (cold and warm).
- `spike-core` release binary size (stripped).
- Webview bundle size (React + xyflow).

## Pass / fail

- ✅ `pong` appears **in a canvas node** in under ~1 s after activation, and survives a VS Code restart.
- ✅ Restricted Mode: activates, no crash, the setting `vnoc.server.path` is ignored.
- ✅ C18, C6, C3 each have a one-paragraph answer below.

## Findings

*Partly done (2026-11-05). Code is built and compiles; GUI checks are still open.*

| Measure | Result |
|---|---|
| `spike-core` release binary | 1.05 MB, **0.87 MB stripped** (`lsp-server` + `tracing-subscriber`) |
| Core alone: spawn → initialize → ping → shutdown | ~3 ms |
| Webview bundle (React + xyflow) | JS 335 KB (109 KB gzip) + CSS 16 KB |
| Extension bundle (with `vscode-languageclient`) | 800 KB, unminified |
| `cargo clippy` with `deny(print_stdout)` | clean |

Still to check by hand (see `spikes/s1-hello-core/README.md`): time to first pong inside VS Code, panel restore after restart, Restricted Mode,
C18 (watching above the folder), C6 (`#:schema`), C3 (`async-lsp` comparison).

Next: [`02-python-driver.md`](02-python-driver.md)
