import * as vscode from "vscode";
import * as path from "path";
import { LanguageClient, TransportKind } from "vscode-languageclient/node";
import { OutlineNode, idsFor, textHash } from "./ids";
import { removeToTrash, restoreFromTrash } from "./trash";

// ───────────────────────────────────────────────────────────────────────────────────────────────
// SPIKE ONLY: here the EXTENSION spawns the Python driver directly. In the product the core owns
// every driver and the extension never starts one (decision, S3). This shortcut keeps S3 about
// VS Code behaviour (WorkspaceEdit, the save hook) instead of about the core.
// ───────────────────────────────────────────────────────────────────────────────────────────────
class SpikeClient extends LanguageClient {
  // The driver's handshake wants `initialize { protocol: 1 }`; vscode-languageclient doesn't send it.
  protected override fillInitializeParams(params: any): void {
    super.fillInitializeParams(params);
    params.protocol = 1;
  }
}

export interface Plan {
  version: number;
  edits: { range: { start: Pos; end: Pos }; newText: string }[];
  written: { id: string; line: number; column: number; span: { start: Pos; end: Pos }; text: string }[];
  skipped: { line: number; column: number; reason: string }[];
  unmatched: unknown[];
  text_hash: string;
  took_us: number;
}
type Pos = { line: number; character: number };

export interface AddResult {
  applied: boolean;
  attempts: number;
  dropped: number; // plans thrown away because the document changed while we were planning
  plan?: Plan;
  ms: number;
}

export interface HookRecord {
  uri: string;
  reason: string;
  ms: number;
  used: boolean; // did the hook return edits?
  edits: number;
  mode: "precomputed" | "atSave";
  timedOut: boolean; // our own budget ran out, so we answered [] instead of letting VS Code time us out
}

const REASONS: Record<number, string> = { 1: "Manual", 2: "AfterDelay", 3: "FocusOut" };
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const cfg = () => vscode.workspace.getConfiguration("vnocS3");

let client: SpikeClient;
let out: vscode.OutputChannel;
const stats = { calls: {} as Record<string, number>, hooks: [] as HookRecord[], reparses: 0, predictedHits: 0, bytesSent: 0 };

async function call<T>(method: string, params: object): Promise<T> {
  stats.calls[method] = (stats.calls[method] ?? 0) + 1;
  stats.bytesSent += JSON.stringify(params).length;
  return client.sendRequest<T>(method, params);
}

export function toTextEdit(e: Plan["edits"][number]): vscode.TextEdit {
  const r = e.range;
  return vscode.TextEdit.insert(new vscode.Position(r.start.line, r.start.character), e.newText);
}

function relPath(doc: vscode.TextDocument): string {
  return vscode.workspace.asRelativePath(doc.uri, false).split(path.sep).join("/");
}

/** Plans on the BUFFER (doc.getText()), not on the file on disk. The version travels with the request and comes back. */
export async function planFor(doc: vscode.TextDocument): Promise<Plan & { msOutline: number; msPlan: number }> {
  const text = doc.getText();
  const version = doc.version;
  const t0 = performance.now();
  const outline = await call<{ nodes: OutlineNode[] }>("vnoc/outline", { text });
  const t1 = performance.now();
  // The extension (standing in for the core) decides WHICH id; the driver decides WHERE it goes.
  const ids = idsFor(relPath(doc), outline.nodes);
  const plan = await call<Plan>("vnoc/planIds", { text, version, ids: ids.map(({ line, column, id }) => ({ line, column, id })) });
  return { ...plan, msOutline: t1 - t0, msPlan: performance.now() - t1 };
}

/** Add IDs to the (possibly dirty) buffer. If the document changed while we planned, drop the plan and retry ONCE. */
export async function addIds(doc: vscode.TextDocument): Promise<AddResult> {
  const t0 = performance.now();
  let dropped = 0;
  for (let attempt = 1; attempt <= 2; attempt++) {
    const plan = await planFor(doc);
    const delay = cfg().get<number>("testDelayMs") ?? 0;
    if (delay) await sleep(delay); // tests use this to "type" while the plan is in flight
    if (doc.version !== plan.version) {
      dropped++;
      out.appendLine(`plan for v${plan.version} dropped: document is at v${doc.version}`);
      continue;
    }
    if (plan.edits.length === 0) return { applied: false, attempts: attempt, dropped, plan, ms: performance.now() - t0 };
    // No `await` between the version check above and applyEdit below, so nothing can slip in.
    const we = new vscode.WorkspaceEdit();
    we.set(doc.uri, plan.edits.map(toTextEdit));
    const ok = await vscode.workspace.applyEdit(we);
    return { applied: ok, attempts: attempt, dropped, plan, ms: performance.now() - t0 };
  }
  return { applied: false, attempts: 2, dropped, ms: performance.now() - t0 };
}

// ── The save hook (VS-D66): plans are computed ahead of time per document version ──────────────
type Cached = { version: number; plan: Promise<Plan> };
const cache = new Map<string, Cached>();
const predicted = new Map<string, string>(); // uri -> hash of the text we expect on disk after this save
const timers = new Map<string, NodeJS.Timeout>();

function precompute(doc: vscode.TextDocument) {
  const version = doc.version;
  cache.set(doc.uri.toString(), { version, plan: planFor(doc) });
}

function registerSaveHook(ctx: vscode.ExtensionContext) {
  ctx.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (!cfg().get<boolean>("autoIds") || e.document.languageId !== "python" || cfg().get<boolean>("computeAtSave")) return;
      const key = e.document.uri.toString();
      // The IDs we add inside a save fire a change event too. If the text is exactly what we predicted, it's ours: no re-plan.
      const want = predicted.get(key);
      if (want && want === textHash(e.document.getText())) return;
      clearTimeout(timers.get(key));
      timers.set(key, setTimeout(() => precompute(e.document), cfg().get<number>("debounceMs") ?? 300));
    }),
    vscode.workspace.onWillSaveTextDocument((e) => {
      if (!cfg().get<boolean>("autoIds") || e.document.languageId !== "python") return;
      const doc = e.document;
      const t0 = performance.now();
      const mode = cfg().get<boolean>("computeAtSave") ? "atSave" : "precomputed";
      const budget = cfg().get<number>("hookBudgetMs") ?? 800;
      let timedOut = false;
      const work = (async () => {
        const slow = cfg().get<number>("slowHookMs") ?? 0;
        if (slow) await sleep(slow);
        let plan: Plan | undefined;
        if (mode === "atSave") {
          plan = await planFor(doc);
        } else {
          const c = cache.get(doc.uri.toString());
          // Use the plan ONLY if it was made for exactly this version of the document.
          if (c && c.version === doc.version) plan = await c.plan;
        }
        return plan && plan.version === doc.version ? plan : undefined;
      })();
      // VS Code gives a save listener 1500 ms and, after 4 timeouts, never calls it again in this session.
      // So we never wait longer than our own budget; a late answer is simply dropped.
      const guarded = budget > 0 ? Promise.race([work, sleep(budget).then(() => { timedOut = true; return undefined; })]) : work;
      e.waitUntil(
        guarded.then((plan) => {
          const edits = plan && !timedOut ? plan.edits.map(toTextEdit) : [];
          if (plan && edits.length) predicted.set(doc.uri.toString(), plan.text_hash);
          stats.hooks.push({ uri: doc.uri.toString(), reason: REASONS[e.reason] ?? String(e.reason), ms: performance.now() - t0, used: edits.length > 0, edits: edits.length, mode, timedOut });
          return edits;
        }),
      );
    }),
    vscode.workspace.onDidSaveTextDocument(async (doc) => {
      // No-loop check: if what landed on disk is exactly what we predicted, the core has nothing to re-parse.
      const key = doc.uri.toString();
      const want = predicted.get(key);
      if (want && want === textHash(doc.getText())) {
        stats.predictedHits++;
        predicted.delete(key);
      } else if (cfg().get<boolean>("autoIds") && doc.languageId === "python") {
        stats.reparses++;
        await call("vnoc/outline", { text: doc.getText() });
      }
    }),
  );
}

export async function activate(ctx: vscode.ExtensionContext) {
  out = vscode.window.createOutputChannel("V-NOC S3");
  const configured = cfg().get<string>("driverPath");
  const bin = configured || path.join(ctx.extensionPath, "..", "s2-python-driver", "spike-driver-python", "target", "release", "spike-driver-python");
  client = new SpikeClient("vnocS3", "V-NOC S3 driver", { command: bin, transport: TransportKind.stdio }, { documentSelector: [] });
  await client.start();
  registerSaveHook(ctx);
  ctx.subscriptions.push(
    vscode.commands.registerCommand("vnoc.s3.addId", async () => {
      const doc = vscode.window.activeTextEditor?.document;
      if (!doc) return;
      const r = await addIds(doc);
      vscode.window.showInformationMessage(`IDs: applied=${r.applied}, attempts=${r.attempts}, dropped=${r.dropped}, ${r.ms.toFixed(0)} ms`);
    }),
    { dispose: () => client.stop() },
  );
  // The test suite drives the extension through this object.
  return { addIds, planFor, call, stats, removeToTrash, restoreFromTrash, precompute, cache };
}

export async function deactivate() {
  await client?.stop();
}
