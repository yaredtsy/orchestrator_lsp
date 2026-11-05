# 06.02: The editor ↔ core protocol

> **Status:** 📝 outline: written in full after **S1** · **Program:** Rust core (+ extension, webview) · **Milestones:** M0–M6

**Purpose:** Every message between the extension and the core: names, shapes, errors, and how TypeScript types are generated from Rust.

**Builds on:** [02-02 running the sidecar](../02-technology/02-running-rust-sidecar.md) · [02-03 message bridge](../02-technology/03-frontend-webview.md) · [plan/06-api](../../plan/06-api/03-application-services.md)

## Will define

1. **Lifecycle:** `initialize` (root, git root, `storageUri`, `isTrusted`, settings), `shutdown`, restart rules.
2. **Requests (MVP):** `tree.children`, `node.get`, `node.code.range`, `callTree`, `callTree.expand`, `docs.list` / `add` / `remove` / `move`, `node.archive` / `relink` / `remove`, `sync.now`, `keys.translate`, `settings.effective`.
3. **Requests (stable):** `ids.add`, `ids.remove`, `ids.forSave` (precomputed), `ids.restore`, `playground.*`.
4. **Notifications:** `graphChanged` (with renamed keys), `missingChanged`, `syncCount`, `progress`, `configError`.
5. **Results that are edits:** `FileOps` / `TextEdit[]` the extension applies (VS-P24); never a path the core already wrote.
6. **Errors and cancellation:** error codes, `$/cancelRequest` for long call trees.
7. **TS generation:** `ts-rs` (or `specta`) from `vnoc-protocol`, checked in CI so Rust and TS can't drift.
8. **Versioning:** free to change until 1.0 (only one machine depends on it), then additive only.

## MVP vs stable

| MVP (0.x) | Stable (1.0) |
|---|---|
| MVP request list | the ID and playground requests |

## Questions to settle while writing

- `ts-rs` or `specta`?
- One `graphChanged` with a diff, or invalidate-and-refetch?
