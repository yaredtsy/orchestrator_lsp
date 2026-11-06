# 06.05: Sync, HEAD awareness, and the matching ladder

> **Status:** 📝 outline: written in full after **S4** · **Program:** Rust core · **Milestones:** M4

**Purpose:** How the core turns file and git events into an updated graph, and how documented nodes are found again or marked Missing.

**Builds on:** [02-05 §4–6](../02-technology/05-identity-and-archive.md) · [plan/05 orchestrator](../../plan/05-orchestrator/00-overview.md)

## Will define

1. **The scheduler:** event batching (~200 ms), auto vs manual, the startup mtime + size scan, one writer.
2. **HEAD awareness:** reading HEAD with gix, the ancestry test, the ref check (is the old HEAD still a branch tip?, VS-D72), `last_synced_head`, rebase/merge in progress.
3. **The ladder:** the six steps as pure functions, the "before" state (cache, or `node.toml` anchors on a fresh clone: steps 1 and 6 only), the threshold from S4.
4. **Node states:** Active / Not on this branch / Missing (computed) and Archived (stored), and every transition.
5. **Two IDs on one node:** strong match → automatic merge (shared beats local, smaller ID wins); weak match → ask (stable).
6. **`key_moves`:** what sync records so restored canvas tabs can translate their keys.
7. **Test plan:** the refactor fixtures from S4 as table-driven tests.

## MVP vs stable

| MVP (0.x) | Stable (1.0) |
|---|---|
| all of it except weak-match prompts | weak-match prompts; in-code IDs become ladder step 0 |

## Questions to settle while writing

- Does the ladder run per sync for every documented node, or only for those that disappeared?
