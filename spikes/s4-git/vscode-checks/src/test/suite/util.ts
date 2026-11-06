import * as fs from "fs";
import * as path from "path";
import { spawnSync } from "child_process";

export const WS = process.env.S4_WS!;
export const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const FINDINGS = process.env.S4_FINDINGS!;
export const findings: Record<string, unknown> = {};
export function saveFindings() {
  let old: Record<string, unknown> = {};
  try { old = JSON.parse(fs.readFileSync(FINDINGS, "utf8")); } catch { /* first run */ }
  fs.writeFileSync(FINDINGS, JSON.stringify({ ...old, ...findings }, null, 2));
}

export function git(...args: string[]): { ok: boolean; out: string } {
  const r = spawnSync("git", ["-C", WS, ...args], { env: { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@t", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@t", GIT_EDITOR: "true" } });
  return { ok: r.status === 0, out: (r.stdout.toString() + r.stderr.toString()).trim() };
}

export const abs = (rel: string) => path.join(WS, rel);
export const disk = (rel: string) => fs.readFileSync(abs(rel), "utf8");
