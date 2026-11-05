import * as path from "path";
import Mocha from "mocha";

export function run(): Promise<void> {
  const mocha = new Mocha({ ui: "tdd", timeout: 180_000, color: false });
  mocha.addFile(path.resolve(__dirname, "s3.test.js"));
  return new Promise((resolve, reject) =>
    mocha.run((failures) => (failures ? reject(new Error(`${failures} tests failed`)) : resolve())),
  );
}
