import * as vscode from "vscode";
import * as path from "path";
import { LanguageClient, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient;
let t0 = Date.now();
// Resolves once the core has started. Anything that talks to the core awaits this first.
let ready: Promise<void>;

// Where the Rust binary is. In Restricted Mode (untrusted folder) we must NOT take the
// path from workspace settings, or a malicious repo could make us run any program.
// `restrictedConfigurations` in package.json already makes VS Code ignore the workspace
// value; the isTrusted check is a second, visible safety net.
function serverPath(ctx: vscode.ExtensionContext): string {
  const configured = vscode.workspace.getConfiguration("vnoc").get<string>("server.path");
  if (configured && vscode.workspace.isTrusted) return configured;
  return path.join(ctx.extensionPath, "..", "spike-core", "target", "release", "spike-core");
}

export function activate(ctx: vscode.ExtensionContext) {
  t0 = Date.now();
  const out = vscode.window.createOutputChannel("V-NOC S1");
  out.appendLine(`workspace.isTrusted = ${vscode.workspace.isTrusted}`);

  client = new LanguageClient(
    "vnocS1",
    "V-NOC S1 core",
    { command: serverPath(ctx), transport: TransportKind.stdio },
    // No documentSelector: we are not a language server, we only use the JSON-RPC pipe.
    { documentSelector: [] },
  );
  // Do NOT await here: VS Code is waiting for activate() to register the serializer below.
  ready = client.start().then(async () => {
    const first = await client.sendRequest<{ pong: boolean; pid: number }>("vnoc/ping");
    out.appendLine(`first pong after ${Date.now() - t0} ms: ${JSON.stringify(first)}`);
  });

  ctx.subscriptions.push(
    vscode.commands.registerCommand("vnoc.s1.ping", async () => {
      await ready;
      const r = await client.sendRequest<{ pong: boolean; pid: number }>("vnoc/ping");
      vscode.window.showInformationMessage(`pong from pid ${r.pid}`);
    }),
    vscode.commands.registerCommand("vnoc.s1.openCanvas", () => {
      const panel = vscode.window.createWebviewPanel("vnocS1", "V-NOC S1", vscode.ViewColumn.Beside, {
        enableScripts: true,
        localResourceRoots: [vscode.Uri.joinPath(ctx.extensionUri, "..", "webview", "dist")],
      });
      setupPanel(ctx, panel);
    }),
    // Step 5: when VS Code restarts it calls this to rebuild panels that were open.
    vscode.window.registerWebviewPanelSerializer("vnocS1", {
      async deserializeWebviewPanel(panel) {
        setupPanel(ctx, panel);
      },
    }),
    // Step 7 (C18): watch a folder ABOVE the opened workspace folder.
    ...watchAbove(out),
  );
}

function watchAbove(out: vscode.OutputChannel): vscode.Disposable[] {
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) return [];
  const repoRoot = vscode.Uri.joinPath(folder.uri, ".."); // opened `repo/sub` -> `repo`
  const w = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(repoRoot, ".vnoc/**"));
  const log = (kind: string) => (u: vscode.Uri) => out.appendLine(`C18 watcher ${kind}: ${u.fsPath}`);
  return [w, w.onDidCreate(log("create")), w.onDidChange(log("change")), w.onDidDelete(log("delete"))];
}

function setupPanel(ctx: vscode.ExtensionContext, panel: vscode.WebviewPanel) {
  const dist = vscode.Uri.joinPath(ctx.extensionUri, "..", "webview", "dist");
  const asUri = (f: string) => panel.webview.asWebviewUri(vscode.Uri.joinPath(dist, f));
  const nonce = Math.random().toString(36).slice(2);
  // CSP: no remote anything. Scripts only with our nonce, styles/fonts only from the extension.
  panel.webview.html = `<!doctype html><html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'nonce-${nonce}'; style-src ${panel.webview.cspSource} 'unsafe-inline'; img-src ${panel.webview.cspSource} data:;">
<link rel="stylesheet" href="${asUri("assets/index.css")}">
</head><body><div id="root"></div>
<script nonce="${nonce}" type="module" src="${asUri("assets/index.js")}"></script></body></html>`;

  // The webview cannot talk to the core directly. It posts a message, we relay it.
  panel.webview.onDidReceiveMessage(async (m: { type: string; id: number; method: string }) => {
    if (m.type !== "req") return;
    await ready;
    const result = await client.sendRequest(m.method);
    panel.webview.postMessage({ type: "res", id: m.id, result, ms: Date.now() - t0 });
  });
}

export async function deactivate() {
  await client?.stop();
}
