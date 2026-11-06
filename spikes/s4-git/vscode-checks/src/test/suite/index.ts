import * as path from "path";
import Mocha from "mocha";

export function run(): Promise<void> {
  const mocha = new Mocha({ ui: "tdd", timeout: 180_000, color: false });
  const only = process.env.S4_ONLY;
  for (const f of ["c9.test.js", "reload.test.js", "formatters.test.js"]) if (!only || f.startsWith(only)) mocha.addFile(path.resolve(__dirname, f));
  return new Promise((resolve, reject) => mocha.run((failures) => (failures ? reject(new Error(`${failures} tests failed`)) : resolve())));
}
