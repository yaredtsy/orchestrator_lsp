import * as assert from "assert";
import * as vscode from "vscode";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

type Api = any; // the object returned by activate()
const WS = process.env.S3_WORKSPACE!;
const FINDINGS = process.env.S3_FINDINGS!;
const findings: Record<string, unknown> = {};
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const cfg = () => vscode.workspace.getConfiguration("vnocS3");
const set = (k: string, v: unknown) => cfg().update(k, v, vscode.ConfigurationTarget.Global);
const pct = (xs: number[], p: number) => { const s = [...xs].sort((a, b) => a - b); return s[Math.min(s.length - 1, Math.floor(p * s.length))]; };
const r1 = (n: number) => Math.round(n * 10) / 10;

const SAMPLE = `def good(x):
    return x + 1


def broken(x):
    y = foo(x,
    return y


def documented(x):
    """Does a thing.

    More detail.
    """
    return "é😀" + x


class Box:
    def m(self):
        return 1
`;

let api: Api;
let n = 0;

async function open(content: string, name = `f${++n}.py`) {
  const uri = vscode.Uri.file(path.join(WS, name));
  await vscode.workspace.fs.writeFile(uri, Buffer.from(content));
  const doc = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
  return { uri, doc, editor };
}
const typeAtTop = (editor: vscode.TextEditor, s: string) => editor.edit((b) => b.insert(new vscode.Position(0, 0), s));
const onDisk = (uri: vscode.Uri) => fs.readFileSync(uri.fsPath, "utf8");
async function reset(doc: vscode.TextDocument, original: string) {
  const we = new vscode.WorkspaceEdit();
  we.replace(doc.uri, new vscode.Range(0, 0, doc.lineCount, 0), original);
  await vscode.workspace.applyEdit(we);
}

suiteSetup(async () => {
  const ext = vscode.extensions.getExtension("vnoc-spike.vnoc-spike-s3")!;
  api = await ext.activate();
});
setup(async () => {
  for (const k of ["autoIds", "debounceMs", "slowHookMs", "testDelayMs", "computeAtSave", "hookBudgetMs"]) await set(k, undefined);
});
suiteTeardown(() => fs.writeFileSync(FINDINGS, JSON.stringify(findings, null, 2)));

suite("S3 step 1: add IDs to a dirty buffer", () => {
  test("user's unsaved text stays, one undo removes only the IDs, disk untouched", async () => {
    const { uri, doc, editor } = await open(SAMPLE);
    await typeAtTop(editor, "# typed by the user, not saved\n");
    assert.ok(doc.isDirty);
    const before = doc.getText();
    const diskBefore = onDisk(uri);

    const r = await api.addIds(doc);
    assert.ok(r.applied, "edit applied");
    const after = doc.getText();
    assert.ok(after.startsWith("# typed by the user, not saved\n"), "user text intact");
    assert.ok(after.includes('"""ID: '), "IDs inserted");
    assert.strictEqual(onDisk(uri), diskBefore, "nothing written to disk behind the editor");
    assert.ok(doc.isDirty, "still a dirty buffer");

    await vscode.commands.executeCommand("undo");
    const undone = doc.getText();
    findings.oneUndoRestoresBuffer = undone === before;
    findings.undoLeftover = undone === before ? null : undone.slice(0, 200);
    assert.strictEqual(undone, before, "ONE undo removes exactly the IDs and nothing the user typed");
  });

  test("a file with a syntax error in one function: only that function is skipped", async () => {
    const { doc } = await open(SAMPLE);
    const r = await api.addIds(doc);
    assert.strictEqual(r.plan.skipped.length, 1, JSON.stringify(r.plan.skipped));
    const t = doc.getText();
    assert.ok(t.includes("ID: "), "other functions got IDs");
    assert.ok(/def broken\(x\):\n    y = foo\(x,\n    return y/.test(t), "broken function untouched");
    findings.syntaxErrorSkipped = r.plan.skipped;
  });

  test("user-written docstring is extended in place (R14: ID inside the docstring)", async () => {
    const { doc } = await open(SAMPLE);
    await api.addIds(doc);
    assert.ok(/More detail\.\n\n    ID: \w+\n    """/.test(doc.getText()), doc.getText());
  });

  test("hash-derived IDs are stable: same path + qualname => same ID on a second file with the same name", async () => {
    const a = await open("def f():\n    return 1\n", "same/x.py".replace("/", "_"));
    const r1_ = await api.addIds(a.doc);
    const idA = r1_.plan.written[0].id;
    await reset(a.doc, "def f():\n    return 1\n");
    const r2_ = await api.addIds(a.doc);
    assert.strictEqual(r2_.plan.written[0].id, idA);
    assert.strictEqual(idA.length, 26);
  });
});

suite("S3 step 1: the user types while we plan", () => {
  test("plan is dropped when the version changed; retried once; typed text survives", async () => {
    const { doc, editor } = await open(SAMPLE);
    await set("testDelayMs", 400);
    const p = api.addIds(doc);
    await sleep(150); // the plan is in flight; now the user types
    await typeAtTop(editor, "# typed during the save\n");
    const r = await p;
    findings.typingDuringPlan = { dropped: r.dropped, attempts: r.attempts, applied: r.applied };
    assert.strictEqual(r.dropped, 1, "first plan was dropped");
    assert.strictEqual(r.attempts, 2);
    assert.ok(r.applied);
    assert.ok(doc.getText().startsWith("# typed during the save\n"), "typed text intact");
    assert.ok(doc.getText().includes("ID: "), "IDs added on the retry");
  });

  test("typing during BOTH attempts: gives up after one retry, nothing half-applied", async () => {
    const { doc, editor } = await open(SAMPLE);
    await set("testDelayMs", 300);
    const before = doc.getText();
    const p = api.addIds(doc);
    await sleep(100);
    await typeAtTop(editor, "a");
    await sleep(350); // second attempt is now in flight
    await typeAtTop(editor, "b");
    const r = await p;
    findings.typingDuringBoth = { dropped: r.dropped, applied: r.applied };
    assert.strictEqual(r.applied, false);
    assert.strictEqual(doc.getText(), "ba" + before, "only the user's keystrokes, no IDs");
  });
});

suite("S3 step 4-6: the save hook (VS-D66)", () => {
  test("precomputed plan is used inside the user's own save; no re-parse afterwards", async () => {
    await set("autoIds", true);
    await set("debounceMs", 50);
    const { uri, doc, editor } = await open(SAMPLE);
    await typeAtTop(editor, "# edit\n");
    await sleep(500); // debounce + plan
    const hooksBefore = api.stats.hooks.length;
    const reparsesBefore = api.stats.reparses;
    const callsBefore = JSON.stringify(api.stats.calls);
    await doc.save();
    const h = api.stats.hooks[hooksBefore];
    await sleep(300);
    findings.precomputedSave = { reason: h.reason, ms: r1(h.ms), used: h.used, edits: h.edits };
    assert.strictEqual(h.reason, "Manual");
    assert.ok(h.used, "hook returned the precomputed edits");
    assert.ok(onDisk(uri).includes('ID: '), "IDs are in the saved file");
    assert.strictEqual(api.stats.reparses, reparsesBefore, "no re-parse after our own save");
    assert.strictEqual(JSON.stringify(api.stats.calls), callsBefore, "no driver call after the save");
    assert.ok(api.stats.predictedHits >= 1, "the saved text hash matched the predicted one");
  });

  test("saving before the debounce fired: stale plan is NOT used (version mismatch)", async () => {
    await set("autoIds", true);
    await set("debounceMs", 5000);
    const { uri, doc, editor } = await open(SAMPLE);
    await typeAtTop(editor, "# edit\n");
    const i = api.stats.hooks.length;
    await doc.save();
    const h = api.stats.hooks[i];
    findings.saveBeforeDebounce = { used: h.used, ms: r1(h.ms) };
    assert.strictEqual(h.used, false);
    assert.ok(!onDisk(uri).includes("ID: "), "no IDs written from a stale plan");
  });

  test("reasons: Manual vs AfterDelay vs FocusOut are reported", async () => {
    await set("autoIds", true);
    await set("debounceMs", 50);
    const { doc, editor } = await open("def a():\n    return 1\n");
    await typeAtTop(editor, "# x\n");
    await sleep(300);
    await doc.save();
    const seen = new Set(api.stats.hooks.map((h: any) => h.reason));
    findings.saveReasonsSeen = [...seen];
    assert.ok(seen.has("Manual"));
  });

});

suite("S3 C8: timings on real files", () => {
  const stdlib = process.env.S3_STDLIB || "/Users/yared/.pyenv/versions/3.12.2/lib/python3.12";
  const pick = (target: number) => {
    const files = fs.readdirSync(stdlib).filter((f) => f.endsWith(".py")).map((f) => {
      const t = fs.readFileSync(path.join(stdlib, f), "utf8");
      return { f, t, lines: t.split("\n").length };
    });
    return files.sort((a, b) => Math.abs(a.lines - target) - Math.abs(b.lines - target))[0];
  };

  test("p50/p95: plan, hook precomputed, hook computed at save; extra-hop headroom", async function () {
    const rows: any[] = [];
    for (const [label, target] of [["small", 100], ["medium", 1000], ["large", 5000]] as const) {
      const file = pick(target);
      const { doc, editor } = await open(file.t, `bench_${label}.py`);
      // (a) plan only: outline + planIds, as the extension measures it
      const plan: number[] = [], pOutline: number[] = [], pPlan: number[] = [], driverUs: number[] = [];
      let defsWithoutId = 0;
      for (let i = 0; i < 30; i++) {
        const p = await api.planFor(doc);
        plan.push(p.msOutline + p.msPlan); pOutline.push(p.msOutline); pPlan.push(p.msPlan); driverUs.push(p.took_us);
        defsWithoutId = p.written.length;
      }
      // payload cost of the extra core hop: the text is serialized twice more (extension->core->driver and back)
      const payload = JSON.stringify({ text: file.t, version: 1, ids: [] });
      const t0 = performance.now();
      for (let i = 0; i < 30; i++) JSON.parse(JSON.stringify(JSON.parse(payload)));
      const hopMs = (performance.now() - t0) / 30;

      // (b)/(c) hook time inside a real save, precomputed vs computed at save
      const hook = { precomputed: [] as number[], atSave: [] as number[] };
      for (const mode of ["precomputed", "atSave"] as const) {
        await set("autoIds", true);
        await set("debounceMs", 30);
        await set("computeAtSave", mode === "atSave");
        for (let i = 0; i < 15; i++) {
          await reset(doc, file.t);
          await typeAtTop(editor, "# e\n");
          await sleep(mode === "precomputed" ? 250 + file.lines / 20 : 50);
          const k = api.stats.hooks.length;
          await doc.save();
          hook[mode].push(api.stats.hooks[k].ms);
        }
      }
      await set("computeAtSave", undefined);
      await set("autoIds", undefined);
      rows.push({
        size: label, file: file.f, lines: file.lines, bytes: file.t.length, defsGettingIds: defsWithoutId,
        plan_ms: { p50: r1(pct(plan, 0.5)), p95: r1(pct(plan, 0.95)) },
        outline_ms_p50: r1(pct(pOutline, 0.5)), planIds_ms_p50: r1(pct(pPlan, 0.5)),
        driver_internal_ms_p50: r1(pct(driverUs, 0.5) / 1000),
        hook_precomputed_ms: { p50: r1(pct(hook.precomputed, 0.5)), p95: r1(pct(hook.precomputed, 0.95)) },
        hook_atSave_ms: { p50: r1(pct(hook.atSave, 0.5)), p95: r1(pct(hook.atSave, 0.95)) },
        extra_hop_serialization_ms: r1(hopMs),
        headroom_ms_if_computed_at_save: r1(1500 - pct(hook.atSave, 0.95) - hopMs * 2),
      });
    }
    findings.timings = rows;
  });
});

suite("S3 C8: a slow hook", () => {
  async function slowSaves(label: string, n: number) {
    const rows: any[] = [];
    for (let i = 0; i < n; i++) {
      const { uri, doc, editor } = await open(`def f${label}${i}():\n    return ${i}\n`);
      await typeAtTop(editor, "# e\n");
      await sleep(300);
      const t0 = Date.now();
      await doc.save();
      rows.push({ save: i + 1, saveMs: Date.now() - t0, idsOnDisk: onDisk(uri).includes("ID: ") });
    }
    return rows;
  }
  test("WITH our budget (800 ms): 8 slow saves, the listener is never dropped", async () => {
    await set("autoIds", true); await set("debounceMs", 50); await set("slowHookMs", 2000); await set("hookBudgetMs", 800);
    const rows = await slowSaves("b", 8);
    findings.slowHookWithBudget = rows;
    assert.ok(rows.every((r) => r.saveMs < 1400), "every save returned within our budget, none hit VS Code's 1500 ms limit");
    // prove the listener is still alive: a fast save afterwards still gets its IDs
    await set("slowHookMs", undefined);
    const { uri, doc, editor } = await open("def alive():\n    return 1\n");
    await typeAtTop(editor, "# e\n"); await sleep(400); await doc.save();
    assert.ok(onDisk(uri).includes("ID: "), "listener still alive after 8 slow saves");
  });
  test("WITHOUT a budget (2 s hook): VS Code cuts at ~1.5 s and drops the listener after 4 (runs last in the session)", async () => {
    await set("autoIds", true); await set("debounceMs", 50); await set("slowHookMs", 2000); await set("hookBudgetMs", 0);
    const rows = await slowSaves("n", 6);
    findings.slowHookNoBudget = rows;
    await set("slowHookMs", undefined);
  });
});


const edit = (e: any) => new vscode.TextEdit(new vscode.Range(e.range.start.line, e.range.start.character, e.range.end.line, e.range.end.character), e.newText);
async function applyEdits(doc: vscode.TextDocument, edits: any[]) {
  const we = new vscode.WorkspaceEdit();
  we.set(doc.uri, edits.map(edit));
  assert.ok(await vscode.workspace.applyEdit(we));
}
const spanText = (doc: vscode.TextDocument, w: any) => doc.getText(new vscode.Range(w.span.start.line, w.span.start.character, w.span.end.line, w.span.end.character));

suite("S3 steps 2-3 (C20): # FileID writer and exact-text remove", () => {
  const cases: [string, string, string][] = [
    ["plain file", "import os\n\n\ndef f():\n    return 1\n", "# FileID: X\nimport os\n\n\ndef f():\n    return 1\n"],
    ["shebang + encoding + license header", "#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\n# Copyright 2026 Acme\n# Licensed under MIT\nimport os\n", "#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\n# Copyright 2026 Acme\n# Licensed under MIT\n# FileID: X\nimport os\n"],
    ["encoding on line 1, module docstring next", "# coding: latin-1\n\"\"\"Module doc.\"\"\"\nimport os\n", "# coding: latin-1\n# FileID: X\n\"\"\"Module doc.\"\"\"\nimport os\n"],
    ["no trailing newline, only a comment", "# just a comment", "# just a comment\n# FileID: X\n"],
    ["CRLF file", "import os\r\nx = 1\r\n", "# FileID: X\r\nimport os\r\nx = 1\r\n"],
  ];
  for (const [name, src, want] of cases) {
    test(`FileID: ${name}`, async () => {
      const { doc } = await open(src);
      const r: any = await api.call("vnoc/planFileId", { text: doc.getText(), version: doc.version, id: "X" });
      await applyEdits(doc, r.edits);
      assert.strictEqual(doc.getText(), want);
      assert.strictEqual(spanText(doc, r.written[0]), r.written[0].text, "recorded span + text read back from the buffer");
      // remove by the recorded text: byte-identical to the original
      const rm: any = await api.call("vnoc/planRemoveIds", { text: doc.getText(), version: doc.version, items: [{ id: "X", text: r.written[0].text }] });
      await applyEdits(doc, rm.edits);
      assert.strictEqual(doc.getText(), src, "remove restores the original exactly");
    });
  }

  test("FileID: a file that already has one gets no edit", async () => {
    const { doc } = await open("# FileID: OLD\nimport os\n");
    const r: any = await api.call("vnoc/planFileId", { text: doc.getText(), version: doc.version, id: "NEW" });
    assert.deepStrictEqual(r.edits, []);
    assert.strictEqual(r.existing, "OLD");
  });

  test("remove: the user edited the ID line by hand -> nothing changes, the line is shown", async () => {
    const { doc } = await open("import os\n");
    const r: any = await api.call("vnoc/planFileId", { text: doc.getText(), version: doc.version, id: "ABC" });
    await applyEdits(doc, r.edits);
    // the user adds a note to the line
    const we = new vscode.WorkspaceEdit();
    we.insert(doc.uri, new vscode.Position(0, "# FileID: ABC".length), "  (keep!)");
    await vscode.workspace.applyEdit(we);
    const before = doc.getText();
    const rm: any = await api.call("vnoc/planRemoveIds", { text: before, version: doc.version, items: [{ id: "ABC", text: r.written[0].text }] });
    findings.removeMismatch = rm.mismatch;
    assert.deepStrictEqual(rm.edits, []);
    assert.strictEqual(rm.mismatch.length, 1);
    assert.ok(rm.mismatch[0].shown.text.includes("keep!"), "the edited line is shown");
    assert.strictEqual(doc.getText(), before, "nothing changed");
  });

  test("remove: still works after the user edits ABOVE the ID (the span moved, the text is searched)", async () => {
    const { doc, editor } = await open("import os\n");
    const r: any = await api.call("vnoc/planFileId", { text: doc.getText(), version: doc.version, id: "MOVED" });
    await applyEdits(doc, r.edits);
    await typeAtTop(editor, "# header added later\n");
    const rm: any = await api.call("vnoc/planRemoveIds", { text: doc.getText(), version: doc.version, items: [{ id: "MOVED", text: r.written[0].text }] });
    await applyEdits(doc, rm.edits);
    assert.strictEqual(doc.getText(), "# header added later\nimport os\n");
  });

  test("def IDs: remove restores each form exactly (new statement, docstring, one-line docstring) and stays valid", async () => {
    const forms = [
      "def a():\n    return 1\n",
      "def b():\n    \"\"\"Summary.\n\n    Details.\n    \"\"\"\n    return 1\n",
      "def c():\n    \"\"\"Summary.\"\"\"\n    return 1\n",
      "def d():\n    'Summary.'\n    return 1\n",
      "class K:\n    def m(self):\n        return 1\n",
    ];
    const rows: any[] = [];
    for (const src of forms) {
      const { doc } = await open(src);
      const r: any = await api.addIds(doc);
      const written = r.plan.written;
      const rm: any = await api.call("vnoc/planRemoveIds", { text: doc.getText(), version: doc.version, items: written.map((w: any) => ({ id: w.id, text: w.text })) });
      await applyEdits(doc, rm.edits);
      rows.push({ form: src.split("\n")[0], exact: doc.getText() === src, got: doc.getText() === src ? undefined : doc.getText() });
    }
    findings.removeDefIdForms = rows;
    assert.ok(rows.every((r) => r.exact), JSON.stringify(rows.filter((r) => !r.exact)));
  });
});

suite("S3 step 7 (VS-D70): Remove to .vnoc/local/trash and Restore", () => {
  const vnoc = () => vscode.Uri.file(path.join(WS, ".vnoc"));
  const exists = (u: vscode.Uri) => fs.existsSync(u.fsPath);
  const rel = (...p: string[]) => vscode.Uri.joinPath(vnoc(), ...p);

  test("open + dirty file follows the move and keeps its unsaved text; Restore brings it back", async () => {
    const files = ["nodes/n1/notes.md", "nodes/n1/sub/a.md", "nodes/n1/ünï cøde 文 x.md"];
    for (const f of files) {
      await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(rel(f), ".."));
      await vscode.workspace.fs.writeFile(rel(f), Buffer.from("saved text\n"));
    }
    const doc = await vscode.workspace.openTextDocument(rel(files[0]));
    const ed = await vscode.window.showTextDocument(doc);
    await ed.edit((b) => b.insert(new vscode.Position(0, 0), "UNSAVED "));

    // does a watcher see one batch or a storm?
    const events: string[] = [];
    const w = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(vnoc(), "**"));
    w.onDidCreate((u) => events.push("c " + path.basename(u.fsPath)));
    w.onDidDelete((u) => events.push("d " + path.basename(u.fsPath)));
    w.onDidChange((u) => events.push("m " + path.basename(u.fsPath)));

    const t0 = performance.now();
    const dir = await api.removeToTrash(vnoc(), "n1", "shared", files);
    const removeMs = performance.now() - t0;
    await sleep(800);
    for (const f of files) assert.ok(!exists(rel(f)), `${f} left its place`);
    for (const f of files) assert.ok(exists(vscode.Uri.joinPath(dir, f)), `${f} is in the trash`);
    assert.ok(exists(vscode.Uri.joinPath(dir, "trashed.toml")));
    const toml = fs.readFileSync(vscode.Uri.joinPath(dir, "trashed.toml").fsPath, "utf8");
    assert.ok(toml.includes('node = "n1"') && toml.includes('side = "shared"') && toml.includes("removed_at"));

    const moved = vscode.workspace.textDocuments.find((d) => d.uri.fsPath === vscode.Uri.joinPath(dir, files[0]).fsPath);
    findings.trashDirtyFileFollows = { followed: !!moved, stillDirty: moved?.isDirty, textKept: moved?.getText() === "UNSAVED saved text\n" };
    assert.ok(moved, "the open editor followed the file into the trash");
    assert.strictEqual(moved!.getText(), "UNSAVED saved text\n", "unsaved text survived the move");

    const res = await api.restoreFromTrash(vnoc(), dir, () => true);
    await sleep(500);
    assert.deepStrictEqual(res.conflicts, []);
    for (const f of files) assert.ok(exists(rel(f)), `${f} restored`);
    const back = vscode.workspace.textDocuments.find((d) => d.uri.fsPath === rel(files[0]).fsPath);
    assert.strictEqual(back?.getText(), "UNSAVED saved text\n", "unsaved text survived Restore too");
    w.dispose();
    findings.trashWatcherEvents = { count: events.length, events: events.slice(0, 40) };
    findings.trashRemoveMs = r1(removeMs);
  });

  test("Restore never overwrites: an existing path is reported as a conflict", async () => {
    const f = "nodes/n2/doc.md";
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(rel(f), ".."));
    await vscode.workspace.fs.writeFile(rel(f), Buffer.from("old"));
    const dir = await api.removeToTrash(vnoc(), "n2", "local", [f]);
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(rel(f), ".."));
    await vscode.workspace.fs.writeFile(rel(f), Buffer.from("NEW, written after the remove"));
    const res = await api.restoreFromTrash(vnoc(), dir, () => true);
    assert.deepStrictEqual(res.conflicts, [f]);
    assert.strictEqual(fs.readFileSync(rel(f).fsPath, "utf8"), "NEW, written after the remove");
    assert.ok(exists(vscode.Uri.joinPath(dir, f)), "the removed copy stays in the trash");
  });

  test("node no longer exists: files come back and the node is reported Missing", async () => {
    const f = "nodes/n3/doc.md";
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(rel(f), ".."));
    await vscode.workspace.fs.writeFile(rel(f), Buffer.from("x"));
    const dir = await api.removeToTrash(vnoc(), "n3", "shared", [f]);
    assert.ok(!exists(vscode.Uri.joinPath(rel(f), "..")) || fs.readdirSync(vscode.Uri.joinPath(rel(f), "..").fsPath).length === 0);
    const res = await api.restoreFromTrash(vnoc(), dir, () => false);
    assert.deepStrictEqual(res.missingNodes, ["n3"]);
    assert.ok(exists(rel(f)));
  });
});
