import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Fixed file names (no hashes) so the extension can reference assets/index.js and assets/index.css directly.
// One file: the CSP only lets our nonce'd script run, so Shiki's grammars (dynamic imports) are inlined instead of split into chunks.
// No index.html: the extension writes the page itself (with the CSP and the nonce).
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist",
    rollupOptions: { input: "src/main.tsx", output: { inlineDynamicImports: true, entryFileNames: "assets/index.js", assetFileNames: "assets/index.[ext]" } },
  },
});
