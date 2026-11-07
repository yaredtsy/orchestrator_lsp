import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { LanguageClient, TransportKind } from "vscode-languageclient/node";

// The core is a separate process: this extension starts it (vscode-languageclient is only a very good JSON-RPC-over-stdio
// client here, we use its custom-request API and none of the language features) and relays the webview's requests to it.
// The extension never starts a driver: the core owns the drivers.

let client: LanguageClient | undefined;
/** Resolves when the core is running. Everything that talks to it awaits this first (S1: never block activate()). */
let ready: Promise<void>;
let output: vscode.OutputChannel;

/** What the webview last said it drew (for the test suite). */
let rendered: { visible: number; hidden: number } | undefined;
const renderWaiters: ((r: { visible: number; hidden: number }) => void)[] = [];

const PANEL = "vnoc.canvas";

/**
 * A binary path: the setting if set, else the build folder of this repository (development layout: `extension/` sits next to
 * `target/` and `drivers/`). In Restricted Mode VS Code hands us the default instead of a workspace value
 * (`restrictedConfigurations` in package.json), so a cloned repo cannot choose which program we run.
 */
function binary(ctx: vscode.ExtensionContext, setting: string, devPath: string[]): string {
  const configured = vscode.workspace.getConfiguration("vnoc").get<string>(setting);
  return configured || path.join(ctx.extensionPath, "..", ...devPath);
}

async function startCore(ctx: vscode.ExtensionContext): Promise<void> {
  const core = binary(ctx, "server.path", ["target", "debug", "vnoc"]);
  const driver = binary(ctx, "driver.path", ["drivers", "python", "target", "debug", "vnoc-driver-python"]);
  if (!fs.existsSync(core)) {
    throw new Error(`The V-NOC core is not at ${core}. Build it: cargo build -p vnoc`);
  }
  client = new LanguageClient(
    "vnoc",
    "V-NOC core",
    { command: core, transport: TransportKind.stdio },
    // not a language server: no document selector. The driver's path travels to the core in `initializationOptions`.
    { documentSelector: [], outputChannel: output, initializationOptions: { driverPath: driver } },
  );
  await client.start();
  output.appendLine(`core started: ${core}\ndriver: ${driver}`);
}

async function request<T>(method: string, params?: object): Promise<T> {
  await ready;
  return client!.sendRequest<T>(method, params);
}

export function activate(ctx: vscode.ExtensionContext) {
  output = vscode.window.createOutputChannel("V-NOC");
  ready = startCore(ctx);
  ready.catch((e) => {
    output.appendLine(String(e));
    vscode.window.showErrorMessage(String(e.message ?? e));
  });

  // Registered synchronously, before anything is awaited: on restart VS Code waits for this serializer (S1).
  ctx.subscriptions.push(
    vscode.commands.registerCommand("vnoc.openCanvas", () => {
      const panel = vscode.window.createWebviewPanel(PANEL, "V-NOC canvas", vscode.ViewColumn.Active, panelOptions(ctx));
      setupPanel(ctx, panel);
    }),
    vscode.window.registerWebviewPanelSerializer(PANEL, {
      async deserializeWebviewPanel(panel) {
        panel.webview.options = panelOptions(ctx);
        setupPanel(ctx, panel);
      },
    }),
    { dispose: () => client?.stop() },
  );

  // What the test suite drives.
  return {
    ping: () => request<{ pong: boolean; pid: number; version: string }>("vnoc/ping"),
    structure: (root: string) => request<any>("vnoc/structure", { root }),
    /** Resolves when a webview reports that it drew the structure (immediately if one already has). */
    waitRendered: (timeoutMs: number) =>
      new Promise<{ visible: number; hidden: number }>((resolve, reject) => {
        if (rendered) return resolve(rendered);
        const t = setTimeout(() => reject(new Error("the webview did not report a drawing")), timeoutMs);
        renderWaiters.push((r) => (clearTimeout(t), resolve(r)));
      }),
  };
}

function distUri(ctx: vscode.ExtensionContext): vscode.Uri {
  return vscode.Uri.joinPath(ctx.extensionUri, "..", "webview", "dist");
}

function panelOptions(ctx: vscode.ExtensionContext): vscode.WebviewPanelOptions & vscode.WebviewOptions {
  return { enableScripts: true, retainContextWhenHidden: true, localResourceRoots: [distUri(ctx)] };
}

function setupPanel(ctx: vscode.ExtensionContext, panel: vscode.WebviewPanel) {
  const dist = distUri(ctx);
  const asUri = (f: string) => panel.webview.asWebviewUri(vscode.Uri.joinPath(dist, f));
  const nonce = Math.random().toString(36).slice(2) + Math.random().toString(36).slice(2);
  // CSP: no remote anything. Scripts only with our nonce, styles and fonts only from the extension.
  panel.webview.html = `<!doctype html><html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'nonce-${nonce}'; style-src ${panel.webview.cspSource} 'unsafe-inline'; img-src ${panel.webview.cspSource} data:; font-src ${panel.webview.cspSource};">
<link rel="stylesheet" href="${asUri("assets/index.css")}">
</head><body><div id="root"></div>
<script nonce="${nonce}" type="module" src="${asUri("assets/index.js")}"></script></body></html>`;

  // The webview cannot reach the core. It posts a request; we relay it and post the answer back.
  panel.webview.onDidReceiveMessage(async (m: { type: string; id: number; method: string; visible?: number; hidden?: number }) => {
    if (m.type === "rendered") {
      rendered = { visible: m.visible ?? 0, hidden: m.hidden ?? 0 };
      output.appendLine(`webview drew ${rendered.visible} nodes (${rendered.hidden} hidden)`);
      renderWaiters.splice(0).forEach((w) => w(rendered!));
      return;
    }
    if (m.type !== "req") return;
    try {
      let result: unknown;
      if (m.method === "structure") {
        const folder = vscode.workspace.workspaceFolders?.[0];
        if (!folder) throw new Error("Open a folder first: V-NOC shows the code of the folder you opened.");
        result = await request("vnoc/structure", { root: folder.uri.fsPath });
      } else if (m.method === "ping") {
        result = await request("vnoc/ping");
      } else {
        throw new Error(`unknown request ${m.method}`);
      }
      panel.webview.postMessage({ type: "res", id: m.id, result });
    } catch (e) {
      panel.webview.postMessage({ type: "res", id: m.id, error: String((e as Error).message ?? e) });
    }
  });
}

export async function deactivate() {
  await client?.stop();
}
