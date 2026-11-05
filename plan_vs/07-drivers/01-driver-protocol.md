# 07.01: The core ↔ driver protocol

> **Status:** 📝 outline: written in full after **S2** · **Program:** driver protocol · **Milestones:** M1

**Purpose:** The contract every language driver implements, versioned so drivers ship separately (VS-D29.1).

**Builds on:** [02-07 §2](../02-technology/07-repo-and-drivers.md) · [plan/04-03 jsonrpc driver](../../plan/04-drivers-and-ids/03-jsonrpc-driver.md)

## Will define

1. **Handshake:** `initialize { protocol }` → `{ protocol, name, version, capabilities }`; refusal rules.
2. **Requests:** `outline`, `callTree`, `expand`, `parseErrors`, `environmentInputs`, `canCarry` (stable: `ids.plan`, `ids.read` with spans, `ids.removeEdit`, `ids.fileHeader`).
3. **Edits, not writes:** a driver returns `TextEdit`s and spans; it never writes a file.
4. **Hygiene:** stdout is the protocol only; logs on stderr; crash → the core restarts it.
5. **Versioning:** additive changes keep the number; breaking ones bump it; the core's supported range.

## MVP vs stable

| MVP (0.x) | Stable (1.0) |
|---|---|
| outline, call trees, parse errors, environment inputs | the `ids.*` methods |

## Questions to settle while writing

- One request per file, or batches?
