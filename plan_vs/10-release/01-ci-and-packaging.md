# 10.01: CI and packaging

> **Status:** 📝 outline: written in full after **S1, S2** · **Program:** release · **Milestones:** M0, ⭐ MVP, ⭐ 1.0

**Purpose:** How a commit becomes a VSIX per platform on both stores, pre-release first.

**Builds on:** [02-06 distribution](../02-technology/06-distribution-and-integrations.md) · [02-07 release workflows](../02-technology/07-repo-and-drivers.md)

## Will define

1. **Targets:** `darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64` (MVP); `linux-arm64` (stable).
2. **Workflows:** build + test on every PR; `release-core` builds the core and the bundled driver, packages with `vsce package --target`, publishes to Marketplace and Open VSX (`--pre-release` for 0.x).
3. **Versioning:** odd minors = pre-release, even = release.
4. **Secrets:** Marketplace and Open VSX tokens.
5. **C5:** which store Cursor and Windsurf install from, checked here.

## MVP vs stable

| MVP (0.x) | Stable (1.0) |
|---|---|
| four platforms, pre-release channel | fifth platform, release channel, a frozen driver protocol |

## Questions to settle while writing

- Cross-compile, or native runners per OS?
