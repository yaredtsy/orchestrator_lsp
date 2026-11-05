# 06.01: Crates and domain types

> **Status:** 📝 outline: written in full after **S1, S2** · **Program:** Rust core · **Milestones:** M0

**Purpose:** The crate layout of the core and the types every other crate shares, so the rest of `06-core/` has names to talk about.

**Builds on:** [02-07 §1 repo layout](../02-technology/07-repo-and-drivers.md) · [plan/02 workspace and crates](../../plan/02-workspace-and-crates/01-cargo-workspace.md) · [plan/01 data model](../../plan/01-data-model/00-overview.md)

## Will define

1. **Crates and their one job each:** `vnoc-domain`, `vnoc-store` (+ `-sqlite`, `-memory`), `vnoc-userdata`, `vnoc-sync`, `vnoc-app`, `vnoc-protocol`, `vnoc-driver-protocol`, `vnoc-driver-host`, the `vnoc` binary. Which crate may depend on which (a picture).
2. **Domain types:** `PermId` (26 chars), `LocationKey` (`path::qualname`), `RepoPath` (repo-relative, `/` only), `NodeKind`, `Node`, `NodeToml`, `IdInCode`, `Status`, `Side`, `FileOps`.
3. **Newtypes that make mistakes impossible:** a `RepoPath` can't hold a backslash or an absolute path; a `PermId` can only come from `derive_id` or a parsed file.
4. **Errors:** `thiserror` enums per library crate, `anyhow` only in the binary (plan/P15).
5. **Concurrency model:** the request loop, the one SQLite writer thread, the driver connections. Threads + channels or tokio (decided here, from S1/S2).

## MVP vs stable

| MVP (0.x) | Stable (1.0) |
|---|---|
| all of it (M0) | unchanged; new types for IDs-in-code arrive as fields, not new crates |

## Questions to settle while writing

- tokio or plain threads + channels for the core (S1 compares)?
- Which parts of `plan/` crates are reused as code, which only as ideas?
