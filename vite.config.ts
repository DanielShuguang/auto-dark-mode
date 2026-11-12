import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import UnoCSS from "unocss/vite";

// Tauri 固定端口：devUrl 必须与之匹配，且不能被自动改端口
export default defineConfig({
  plugins: [vue(), UnoCSS()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
