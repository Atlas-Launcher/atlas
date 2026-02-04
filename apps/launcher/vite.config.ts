import { defineConfig } from "vite";
import { fileURLToPath } from "url";
import { dirname, resolve } from "path";
import vue from "@vitejs/plugin-vue";

const rootDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig(() => ({
  plugins: [vue()],
  clearScreen: false,
  resolve: {
    alias: {
      "@": resolve(rootDir, "src")
    }
  },
  server: {
    port: 1420,
    strictPort: true
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2020",
    minify: !process.env.TAURI_DEBUG,
    sourcemap: !!process.env.TAURI_DEBUG
  }
}));
