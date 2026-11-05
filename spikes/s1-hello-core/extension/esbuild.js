// Bundles src/extension.ts into one file. `vscode` is provided by the host, so it is external.
const esbuild = require("esbuild");
const watch = process.argv.includes("--watch");
const opts = {
  entryPoints: ["src/extension.ts"],
  bundle: true,
  outfile: "dist/extension.js",
  external: ["vscode"],
  format: "cjs",
  platform: "node",
  sourcemap: true,
};
(watch ? esbuild.context(opts).then((c) => c.watch()) : esbuild.build(opts)).catch(() => process.exit(1));
