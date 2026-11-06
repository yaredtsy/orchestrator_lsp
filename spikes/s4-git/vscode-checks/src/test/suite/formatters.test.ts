//! Which Python formatters return ONE whole-document edit, and which return minimal edits?
//! Needs the formatter extensions installed in S4_EXTENSIONS_DIR; skipped (and recorded as such) when they are not.

import * as vscode from "vscode";
import * as fs from "fs";
import { abs, findings, saveFindings } from "./util";

suiteTeardown(() => saveFindings());

const UGLY = `import os,sys
def f( a,b ):
    x=a+ b
    if x>3 :
        return   {"k":1,'z':2}
    return [1,2,
        3]


class   K:
    def m(self,y):
        return y*2
`;

const FORMATTERS = ["ms-python.black-formatter", "charliermarsh.ruff", "ms-python.autopep8", "eeyore.yapf"];

suite("Formatters: the shape of the edits they return", () => {
  test("measure each installed formatter", async function () {
    const rows: Record<string, unknown> = {};
    await vscode.workspace.getConfiguration("python").update("defaultInterpreterPath", process.env.S4_PYTHON ?? "/Users/yared/.pyenv/versions/3.12.2/bin/python", vscode.ConfigurationTarget.Global);
    for (const id of FORMATTERS) {
      const ext = vscode.extensions.getExtension(id);
      if (!ext) {
        rows[id] = "not installed";
        continue;
      }
      try {
        await ext.activate();
      } catch (e) {
        rows[id] = `failed to activate: ${e}`;
        continue;
      }
      const rel = `py/fmt_${id.replace(/\W/g, "_")}.py`;
      fs.writeFileSync(abs(rel), UGLY);
      const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(abs(rel)));
      await vscode.window.showTextDocument(doc);
      await vscode.workspace.getConfiguration().update("[python]", { "editor.defaultFormatter": id }, vscode.ConfigurationTarget.Global);
      let edits: vscode.TextEdit[] | undefined;
      for (let i = 0; i < 10 && !(edits && edits.length); i++) {
        edits = await vscode.commands.executeCommand<vscode.TextEdit[]>("vscode.executeFormatDocumentProvider", doc.uri, { tabSize: 4, insertSpaces: true });
        if (!edits?.length) await new Promise((r) => setTimeout(r, 1000));
      }
      const full = new vscode.Range(0, 0, doc.lineCount, 0);
      rows[id] = {
        version: ext.packageJSON.version,
        edits: edits?.length ?? 0,
        wholeDocument: !!edits && edits.length === 1 && edits[0].range.start.line === 0 && edits[0].range.end.line >= doc.lineCount - 1,
        ranges: (edits ?? []).slice(0, 6).map((e) => `${e.range.start.line}:${e.range.start.character}-${e.range.end.line}:${e.range.end.character} (${e.newText.length} chars)`),
        documentLines: doc.lineCount,
        fullRange: `${full.end.line}`,
      };
    }
    findings.formatters = rows;
  });
});
