import * as path from "path";
import * as os from "os";
import * as fs from "fs";
import { spawnSync } from "child_process";
import { runTests } from "@vscode/test-electron";

// A throwaway profile and a pre-built git repo (so the built-in git extension discovers it). Uses the installed VS Code.
// S4_EXTENSIONS_DIR (optional): a folder with formatter extensions already installed, used by the formatter suite.
const git = (dir: string, ...a: string[]) =>
  spawnSync("git", ["-C", dir, ...a], { env: { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@t", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@t" } });
const put = (dir: string, rel: string, text: string) => {
  fs.mkdirSync(path.dirname(path.join(dir, rel)), { recursive: true });
  fs.writeFileSync(path.join(dir, rel), text);
};
const commit = (dir: string, msg: string) => {
  git(dir, "add", "-A");
  git(dir, "commit", "-q", "-m", msg);
};

function buildRepo(ws: string) {
  git(ws, "init", "-q", "-b", "main");
  put(ws, "f.txt", "one\n");
  put(ws, "py/r2.py", "x = 1\n");
  commit(ws, "base");
  git(ws, "checkout", "-q", "-b", "b");
  put(ws, "f.txt", "from b\n");
  commit(ws, "b");
  git(ws, "checkout", "-q", "main");
  git(ws, "checkout", "-q", "-b", "other");
  put(ws, "py/r2.py", "x = 2  # changed on the other branch\n");
  commit(ws, "other");
  git(ws, "checkout", "-q", "main");
  put(ws, "f.txt", "from main\n");
  commit(ws, "main");
}

async function main() {
  const root = path.resolve(__dirname, "../..");
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "vnoc-s4c-"));
  const ws = path.join(tmp, "ws");
  fs.mkdirSync(ws);
  buildRepo(ws);
  await runTests({
    vscodeExecutablePath: "/Applications/Visual Studio Code.app/Contents/MacOS/Code",
    extensionDevelopmentPath: root,
    extensionTestsPath: path.join(root, "out/test/suite/index"),
    launchArgs: [ws, ...(process.env.S4_EXTENSIONS_DIR ? [] : ["--disable-extensions"]), `--user-data-dir=${path.join(tmp, "user")}`, `--extensions-dir=${process.env.S4_EXTENSIONS_DIR ?? path.join(tmp, "ext")}`, "--disable-workspace-trust"],
    extensionTestsEnv: { S4_WS: ws, S4_FINDINGS: path.join(root, "findings.json"), S4_ONLY: process.env.S4_ONLY ?? "" },
  });
}
main().catch((e) => {
  console.error(e);
  process.exit(1);
});
