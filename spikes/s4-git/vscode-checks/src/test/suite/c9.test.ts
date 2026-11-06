//! C9: what does `vscode.git` report during a rebase / merge / cherry-pick, on a branch switch, and with git.enabled = false?
//! Does a plain file watcher on `.git` work as a fallback nudge?

import * as assert from "assert";
import * as vscode from "vscode";
import { WS, findings, git, saveFindings, sleep } from "./util";

type Snap = { head: string | undefined; rebase: string | null; merge: number; work: number; index: number };
const snap = (repo: any): Snap => {
  const s = repo.state;
  return { head: s.HEAD?.name, rebase: s.rebaseCommit?.hash?.slice(0, 7) ?? null, merge: s.mergeChanges.length, work: s.workingTreeChanges.length, index: s.indexChanges.length };
};

let api: any;
let repo: any;
const events: { ms: number; snap: Snap }[] = [];
const watched: { ms: number; what: string }[] = [];
let t0 = Date.now();

suiteSetup(async () => {
  const ext = vscode.extensions.getExtension("vscode.git");
  findings.c9_builtinGitPresent = !!ext;
  assert.ok(ext, "the built-in git extension is present even with --disable-extensions");
  await ext!.activate();
  api = ext!.exports.getAPI(1);
  for (let i = 0; i < 50 && api.repositories.length === 0; i++) await sleep(200);
  assert.ok(api.repositories.length > 0, "vscode.git discovered the repository");
  repo = api.repositories[0];
  repo.state.onDidChange(() => events.push({ ms: Date.now() - t0, snap: snap(repo) }));
  // a plain watcher: the fallback when vscode.git is off
  const w = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(WS, ".git/{HEAD,MERGE_HEAD,CHERRY_PICK_HEAD,rebase-merge,rebase-merge/*,rebase-apply,rebase-apply/*,refs/heads/**}"));
  for (const [k, ev] of [["create", w.onDidCreate], ["change", w.onDidChange], ["delete", w.onDidDelete]] as const) ev((u: vscode.Uri) => watched.push({ ms: Date.now() - t0, what: `${k} ${u.fsPath.replace(WS, "")}` }));
  findings.c9_gitApiState = api.state;
});
suiteTeardown(() => saveFindings());

async function scenario(name: string, run: () => void | Promise<void>, settleMs = 5000) {
  events.length = 0;
  watched.length = 0;
  t0 = Date.now();
  const before = snap(repo);
  await run();
  await sleep(settleMs);
  const r = {
    before,
    after: snap(repo),
    vscodeGitEvents: events.length,
    duringRelevantState: events.map((e) => e.snap).filter((s) => s.rebase || s.merge > 0 || s.head === undefined).slice(0, 2),
    firstEventMs: events[0]?.ms ?? null,
    watcherEvents: watched.length,
    watcherSample: watched.slice(0, 8).map((w) => w.what),
  };
  findings[`c9_${name}`] = r;
  return r;
}

suite("C9: vscode.git during git operations", () => {
  test("branch switch (checkout)", async () => {
    const r = await scenario("checkout", () => { git("checkout", "-q", "other"); });
    assert.ok(r.vscodeGitEvents > 0, "vscode.git nudges on a HEAD change");
    assert.strictEqual(r.after.head, "other");
    git("checkout", "-q", "main");
    await sleep(500);
  });

  test("rebase with a conflict, then abort", async () => {
    git("checkout", "-q", "b");
    await sleep(500);
    const r = await scenario("rebase_conflict", () => { assert.ok(!git("rebase", "main").ok); });
    findings.c9_rebase_rebaseCommitReported = r.after.rebase !== null;
    const a = await scenario("rebase_abort", () => { assert.ok(git("rebase", "--abort").ok); });
    findings.c9_rebase_endNudged = a.vscodeGitEvents > 0;
    git("checkout", "-q", "main");
    await sleep(500);
  });

  test("merge with a conflict, then abort", async () => {
    const r = await scenario("merge_conflict", () => { assert.ok(!git("merge", "b").ok); });
    findings.c9_merge_conflictsReported = r.after.merge > 0;
    const a = await scenario("merge_abort", () => { assert.ok(git("merge", "--abort").ok); });
    findings.c9_merge_endNudged = a.vscodeGitEvents > 0;
  });

  test("cherry-pick with a conflict, then abort", async () => {
    const r = await scenario("cherrypick_conflict", () => { assert.ok(!git("cherry-pick", "b").ok); });
    findings.c9_cherrypick_conflictsReported = r.after.merge > 0;
    const a = await scenario("cherrypick_abort", () => { assert.ok(git("cherry-pick", "--abort").ok); });
    findings.c9_cherrypick_endNudged = a.vscodeGitEvents > 0;
  });
});

suite("C9: git.enabled = false", () => {
  test("what is left of the nudge, and does a watcher on .git take over?", async () => {
    const cfg = vscode.workspace.getConfiguration("git");
    await cfg.update("enabled", false, vscode.ConfigurationTarget.Global);
    await sleep(3000);
    findings.c9_disabled_apiStateAfter = api.state;
    findings.c9_disabled_repositoriesAfter = api.repositories.length;
    events.length = 0;
    watched.length = 0;
    t0 = Date.now();
    git("checkout", "-q", "other");
    await sleep(1500);
    findings.c9_disabled_checkout = { vscodeGitEvents: events.length, watcherEvents: watched.length, watcherSample: watched.slice(0, 6).map((w) => w.what) };
    watched.length = 0;
    git("checkout", "-q", "b");
    assert.ok(!git("rebase", "main").ok);
    await sleep(1500);
    findings.c9_disabled_rebaseStart = { vscodeGitEvents: events.length, watcherSample: watched.slice(0, 8).map((w) => w.what) };
    watched.length = 0;
    git("rebase", "--abort");
    await sleep(1500);
    findings.c9_disabled_rebaseEnd = { watcherSample: watched.slice(0, 8).map((w) => w.what) };
    git("checkout", "-q", "main");
    await cfg.update("enabled", undefined, vscode.ConfigurationTarget.Global);
  });
});
