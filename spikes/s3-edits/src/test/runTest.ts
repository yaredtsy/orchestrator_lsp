import * as path from "path";
import * as os from "os";
import * as fs from "fs";
import { runTests } from "@vscode/test-electron";

// Runs the suite inside a real VS Code extension host, with a throwaway profile so nothing of the
// user's settings is touched. Uses the VS Code that is already installed.
async function main() {
  const root = path.resolve(__dirname, "../..");
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "vnoc-s3-"));
  const workspace = path.join(tmp, "ws");
  fs.mkdirSync(workspace);
  await runTests({
    vscodeExecutablePath: "/Applications/Visual Studio Code.app/Contents/MacOS/Code",
    extensionDevelopmentPath: root,
    extensionTestsPath: path.join(root, "out/test/suite/index"),
    launchArgs: [workspace, "--disable-extensions", `--user-data-dir=${path.join(tmp, "user")}`, `--extensions-dir=${path.join(tmp, "ext")}`, "--disable-workspace-trust"],
    extensionTestsEnv: { S3_FINDINGS: path.join(root, "findings.json"), S3_WORKSPACE: workspace },
  });
}
main().catch((e) => {
  console.error(e);
  process.exit(1);
});
