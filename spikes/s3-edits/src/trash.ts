import * as vscode from "vscode";

// VS-D70: no system trash. Remove moves files to .vnoc/local/trash/<timestamp>_<node-id>/ and writes trashed.toml.
// B8: the MOVES go through WorkspaceEdit.renameFile (a user may have the file open, even dirty);
// trashed.toml is V-NOC's own file, so it is written directly.

const dec = new TextDecoder();
const enc = new TextEncoder();

export interface TrashedFile { node: string; side: "shared" | "local"; file: string; removed_at: string }

/** `files` are paths relative to the .vnoc folder, e.g. "nodes/n1/notes.md". */
export async function removeToTrash(vnoc: vscode.Uri, nodeId: string, side: "shared" | "local", files: string[]) {
  const removed_at = new Date().toISOString();
  const dir = vscode.Uri.joinPath(vnoc, "local", "trash", `${removed_at.replace(/[:.]/g, "-")}_${nodeId}`);
  const we = new vscode.WorkspaceEdit();
  for (const f of files) we.renameFile(vscode.Uri.joinPath(vnoc, f), vscode.Uri.joinPath(dir, f), { overwrite: false });
  const ok = await vscode.workspace.applyEdit(we);
  if (!ok) throw new Error("rename into trash failed");
  const toml = files.map((f) => `[[file]]\nnode = "${nodeId}"\nside = "${side}"\nfile = "${f}"\nremoved_at = "${removed_at}"\n`).join("\n");
  await vscode.workspace.fs.writeFile(vscode.Uri.joinPath(dir, "trashed.toml"), enc.encode(toml));
  return dir;
}

function parseTrashed(text: string): TrashedFile[] {
  return text.split("[[file]]").slice(1).map((blk) => {
    const get = (k: string) => new RegExp(`^${k} = "(.*)"$`, "m").exec(blk)?.[1] ?? "";
    return { node: get("node"), side: get("side") as "shared" | "local", file: get("file"), removed_at: get("removed_at") };
  });
}

/** Moves the files back. Never overwrites: a path that exists again is reported as a conflict. */
export async function restoreFromTrash(vnoc: vscode.Uri, trashDir: vscode.Uri, nodeStillExists: (nodeId: string) => boolean) {
  const entries = parseTrashed(dec.decode(await vscode.workspace.fs.readFile(vscode.Uri.joinPath(trashDir, "trashed.toml"))));
  const restored: string[] = [];
  const conflicts: string[] = [];
  const we = new vscode.WorkspaceEdit();
  for (const e of entries) {
    const target = vscode.Uri.joinPath(vnoc, e.file);
    try {
      await vscode.workspace.fs.stat(target);
      conflicts.push(e.file);
    } catch {
      we.renameFile(vscode.Uri.joinPath(trashDir, e.file), target, { overwrite: false });
      restored.push(e.file);
    }
  }
  if (restored.length) await vscode.workspace.applyEdit(we);
  // If the node is gone, the files still come back; the UI then shows the node as Missing.
  const missingNodes = [...new Set(entries.map((e) => e.node))].filter((n) => !nodeStillExists(n));
  return { restored, conflicts, missingNodes };
}
