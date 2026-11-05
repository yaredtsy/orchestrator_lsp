# 05.03: S3 edits and saving

**Goal:** prove every source and doc change can go through VS Code's document model: undoable, safe on dirty buffers, and part of the user's own save.
**Answers:** B8 (Rust writes only cache, `node.toml`, `.gitignore`), C10 (save-time IDs in manual sync), C16 (detecting "no system trash"), C20 (pylspt span recording and
the `# FileID:` writer). Also confirms the C8 numbers (~1.5 s shared budget) on a real machine.
**Time box:** 2 days. **Code:** `spikes/s3-edits/`. **Needs:** S1.

MVP note: in-code IDs ship in **stable** (M7), but the risky VS Code behaviour is tested now, so the MVP's design doesn't block it.

## Build

1. **Add an ID to a dirty buffer.** Type something unsaved in `orders.py`, then run "Add permanent ID": the core asks pylspt's `plan_ids` (fed with a hash-derived ID, 02-05 §1)
   for the edit, the host applies it with `workspace.applyEdit`. Check: the user's unsaved text is intact, **one** ⌘Z removes only the ID, no "file changed on disk" warning.
2. **C20, `# FileID:`:** prototype the file-level writer in the adapter (after shebang, encoding and a license header). Check the recorded **span + text** come back on the next read.
3. **Exact-span remove:** remove an ID by its recorded span. Then the mismatch case: edit the `ID:` line by hand, try to remove → nothing changes, the line is shown.
4. **Save hook with precomputed edits (VS-D66):** on `onDidChangeTextDocument` (debounced) compute the ID edits for that `document.version`; in `onWillSaveTextDocument`
   return them only if the version matches. Try `Manual`, `FocusOut`, `AfterDelay`; log the `reason` and how long the hook took. Then make the hook slow on purpose
   (2 s): what does VS Code do, and after how many slow saves does it stop calling the listener (C8)?
5. **C10:** turn on manual sync (no indexing on change). Do save-time IDs still work from the one buffer?
6. **No-loop check:** after our save, `onDidSave` → the core sees a content hash it already predicted → no re-parse.
7. **Remove → trash (C16):** `workspace.fs.delete(uri, { useTrash: true })` on macOS, Windows, Linux desktop, and in **Remote SSH, WSL and a dev container**.
   How do we know *before* deleting that there's no trash (the file system provider's capabilities, an error, a probe)? Then the `.vnoc/local/trash/<date>/` fallback.
8. **B8:** list every file the core writes directly vs through `WorkspaceEdit` in this spike. Anything a user could have open must be in the second list.

> **🧩 VS Code note: `WorkspaceEdit` vs writing the file.**
> `fs.writeFile` changes the file on disk *behind* the editor: if the user has it open with unsaved changes, VS Code shows a conflict and the undo stack is gone.
> A `WorkspaceEdit` changes the **document** (the in-memory model the editor shows). It's undoable like typing, and it works the same whether the file is open or not.
> Saving is a separate step (`document.save()`).

## Measure

- Save-hook time for files of 100, 1,000 and 5,000 lines (precomputed vs computed at save).
- How many slow saves until VS Code drops the listener (C8).

## Pass / fail

- ✅ Steps 1–6 work without a disk conflict or a broken undo.
- ✅ C10, C16, C20, B8 have answers below; the trash fallback works in at least one remote environment.

## Findings

*(empty until S3 runs)*

Next: [`04-git-and-matching.md`](04-git-and-matching.md)
