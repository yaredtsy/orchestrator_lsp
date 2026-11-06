//! Step 0 (VS-D76) reload test: how do we tell "a disk reload" (git checkout, an agent writing an open file) from "the user edited"?
//! Candidate rule: not dirty after the change + a watcher event for the same file + buffer text equals disk.

import * as assert from "assert";
import * as fs from "fs";
import * as vscode from "vscode";
import { WS, abs, disk, findings, git, saveFindings, sleep } from "./util";

type Ev = { ms: number; kind: "change" | "watch" | "save"; reason?: string; dirty?: boolean; eqDisk?: boolean; changes?: string };

suiteTeardown(() => saveFindings());

const REASON: Record<number, string> = { 1: "Undo", 2: "Redo" };
const summ = (e: vscode.TextDocumentChangeEvent) =>
  `${e.contentChanges.length} change(s); first: ${e.contentChanges[0] ? `${e.contentChanges[0].range.start.line}:${e.contentChanges[0].range.start.character}-${e.contentChanges[0].range.end.line}:${e.contentChanges[0].range.end.character}, ${e.contentChanges[0].text.length} chars` : "none"}`;

async function observe(rel: string, doStuff: (doc: vscode.TextDocument, ed: vscode.TextEditor, mark: () => void) => Promise<void>) {
  const uri = vscode.Uri.file(abs(rel));
  const doc = await vscode.workspace.openTextDocument(uri);
  const ed = await vscode.window.showTextDocument(doc);
  const ev: Ev[] = [];
  let t0 = Date.now();
  const w = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(WS, rel));
  const subs = [
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (e.document.uri.fsPath !== uri.fsPath) return;
      ev.push({ ms: Date.now() - t0, kind: "change", reason: e.reason ? REASON[e.reason] ?? String(e.reason) : undefined, dirty: e.document.isDirty, eqDisk: e.document.getText() === disk(rel), changes: summ(e) });
    }),
    vscode.workspace.onDidSaveTextDocument((d) => d.uri.fsPath === uri.fsPath && ev.push({ ms: Date.now() - t0, kind: "save", dirty: d.isDirty, eqDisk: d.getText() === disk(rel) })),
    w.onDidChange(() => ev.push({ ms: Date.now() - t0, kind: "watch" })),
    w.onDidCreate(() => ev.push({ ms: Date.now() - t0, kind: "watch" })),
    w,
  ];
  await sleep(500);
  t0 = Date.now();
  await doStuff(doc, ed, () => { t0 = Date.now(); });
  await sleep(1500);
  subs.forEach((s) => s.dispose());
  return { ev, doc, ed };
}

/// The candidate rule: some content change leaves the document NOT dirty and equal to disk, AND a watcher event for the file is seen within `windowMs` of it (before or after).
function looksLikeReload(ev: Ev[], windowMs = 600): boolean {
  return ev.filter((e) => e.kind === "change" && e.dirty === false && e.eqDisk === true).some((c) => ev.some((w) => w.kind === "watch" && Math.abs(w.ms - c.ms) <= windowMs));
}

const record = (name: string, ev: Ev[], expectedReload: boolean) => {
  const got = looksLikeReload(ev);
  findings[`reload_${name}`] = { events: ev, looksLikeReload: got, expectedReload };
  return got;
};

suite("Step 0: the reload test", () => {
  test("R1 an agent writes the open, clean file", async () => {
    fs.writeFileSync(abs("py/r1.py"), "a = 1\n");
    const { ev } = await observe("py/r1.py", async () => { fs.writeFileSync(abs("py/r1.py"), "a = 2\nb = 3\n"); });
    assert.strictEqual(record("R1_external_write_clean", ev, true), true, JSON.stringify(ev));
  });

  test("R2 git checkout changes the open, clean file", async () => {
    git("checkout", "-q", "main");
    const { ev } = await observe("py/r2.py", async () => { assert.ok(git("checkout", "-q", "other").ok); });
    git("checkout", "-q", "main");
    assert.strictEqual(record("R2_git_checkout", ev, true), true, JSON.stringify(ev));
  });

  test("R3 an agent writes a file that has UNSAVED edits (conflict)", async () => {
    fs.writeFileSync(abs("py/r3.py"), "a = 1\n");
    const { ev, doc } = await observe("py/r3.py", async (_d, ed) => {
      await ed.edit((b) => b.insert(new vscode.Position(0, 0), "# typed\n"));
      await sleep(300);
      fs.writeFileSync(abs("py/r3.py"), "agent = 1\n");
    });
    findings.reload_R3_bufferAfter = { text: doc.getText(), dirty: doc.isDirty, disk: disk("py/r3.py") };
    assert.strictEqual(record("R3_external_write_dirty", ev, false), false, JSON.stringify(ev));
  });

  test("R4 type, then undo back to the saved state", async () => {
    fs.writeFileSync(abs("py/r4.py"), "a = 1\n");
    const { ev } = await observe("py/r4.py", async (_d, ed) => {
      await ed.edit((b) => b.insert(new vscode.Position(0, 0), "# typed\n"));
      await sleep(300);
      await vscode.commands.executeCommand("undo");
    });
    assert.strictEqual(record("R4_undo_to_saved", ev, false), false, JSON.stringify(ev));
  });

  test("R5 type, then the user saves", async () => {
    fs.writeFileSync(abs("py/r5.py"), "a = 1\n");
    const { ev } = await observe("py/r5.py", async (d, ed) => {
      await ed.edit((b) => b.insert(new vscode.Position(0, 0), "# typed\n"));
      await sleep(300);
      await d.save();
    });
    // the save itself fires no content change; only the earlier typing does (and that one is dirty)
    assert.strictEqual(record("R5_user_save", ev, false), false, JSON.stringify(ev));
  });

  test("R6 type, then 'Revert File'", async () => {
    fs.writeFileSync(abs("py/r6.py"), "a = 1\n");
    const { ev } = await observe("py/r6.py", async (_d, ed) => {
      await ed.edit((b) => b.insert(new vscode.Position(0, 0), "# typed\n"));
      await sleep(300);
      await vscode.commands.executeCommand("workbench.action.files.revert");
    });
    record("R6_revert_file", ev, false); // recorded, not asserted: this is a measurement
  });

  test("R7 a formatter-style whole-document replace, in the editor", async () => {
    fs.writeFileSync(abs("py/r7.py"), "a=1\nb =  2\n");
    const { ev } = await observe("py/r7.py", async (_d, ed) => {
      await ed.edit((b) => b.replace(new vscode.Range(0, 0, ed.document.lineCount, 0), "a = 1\nb = 2\n"));
    });
    assert.strictEqual(record("R7_editor_whole_document", ev, false), false, JSON.stringify(ev));
  });

  test("R8 the same through WorkspaceEdit (as an extension would)", async () => {
    fs.writeFileSync(abs("py/r8.py"), "a=1\nb =  2\n");
    const { ev } = await observe("py/r8.py", async (d) => {
      const we = new vscode.WorkspaceEdit();
      we.replace(d.uri, new vscode.Range(0, 0, d.lineCount, 0), "a = 1\nb = 2\n");
      await vscode.workspace.applyEdit(we);
    });
    assert.strictEqual(record("R8_workspaceedit_whole_document", ev, false), false, JSON.stringify(ev));
  });
});
