# S1 hello core: how to run

```
spike-core/   Rust binary (lsp-server). Answers `vnoc/ping`.
extension/    VS Code extension (TypeScript, esbuild). Starts spike-core, relays webview messages.
webview/      React + @xyflow/react, built by Vite into webview/dist.
```

## Build (already done once; repeat after edits)

```sh
(cd spike-core && cargo build --release)
(cd webview   && npm run build)
(cd extension && npm run build)
```

## Run

1. `code extension/` (open the **extension** folder as its own window).
2. Press **F5**. A second window opens (the Extension Development Host).
3. In that window: ⇧⌘P → **V-NOC S1: Ping core** (a notification shows the pid), then **V-NOC S1: Open canvas** (one node saying "pong from pid …").
4. View → Output → **V-NOC S1** shows `isTrusted`, the time to the first pong, and C18 watcher events.

## Checks left for a human (need the GUI)

- Quit the Dev Host with the canvas open, press F5 again: does the panel come back? (Step 5)
- Open a folder in Restricted Mode: does it activate, and is `isTrusted = false`? (Step 6)
- C18: in the Dev Host open `repo/sub/`, then `touch repo/.vnoc/x`. Does the Output panel log it? (Step 7)
