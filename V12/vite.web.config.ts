// Web build: same React components, Tauri modules swapped for browser shims at bundle time.
// The desktop build (vite.config.ts) is untouched. Output: dist-web/ (index.html + assets), served by web/server via STATIC_DIR.
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const at = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// The entry is web.html (so it cannot collide with the desktop index.html); publish it as index.html.
const publishAsIndex = (): Plugin => ({
  name: "publish-web-html-as-index",
  enforce: "post", // must run after Vite has emitted the HTML asset
  generateBundle(_, bundle) {
    const page = bundle["web.html"];
    if (!page) throw new Error("web.html was not emitted; cannot publish index.html");
    page.fileName = "index.html";
  },
});

export default defineConfig({
  plugins: [react(), publishAsIndex()],
  resolve: {
    alias: {
      "@tauri-apps/api/core": at("./src/web/shims/core.ts"),
      "@tauri-apps/plugin-dialog": at("./src/web/shims/dialog.ts"),
      "@tauri-apps/plugin-opener": at("./src/web/shims/opener.ts"),
      "@tauri-apps/api/event": at("./src/web/shims/event.ts"),
    },
  },
  build: { outDir: "dist-web", emptyOutDir: true, target: "es2022", chunkSizeWarningLimit: 1200, rollupOptions: { input: at("./web.html") } },
  server: { port: 1421, strictPort: true, proxy: { "/api": "http://127.0.0.1:8080", "/healthz": "http://127.0.0.1:8080" } },
});
