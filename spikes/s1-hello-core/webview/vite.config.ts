import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Fixed file names (no hashes) so the extension can reference assets/index.js directly.
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist",
    rollupOptions: { input: "src/main.tsx", output: { entryFileNames: "assets/index.js", assetFileNames: "assets/index.[ext]" } },
  },
});
