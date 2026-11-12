// 极简静态服务器：把已构建的 dist/ 挂在 127.0.0.1:5173，
// 用于在禁止 esbuild 子进程的环境里替代 vite dev。
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const ROOT = "dist";
const PORT = 5173;
const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
  ".json": "application/json",
};

createServer(async (req, res) => {
  const url = new URL(req.url ?? "/", "http://localhost");
  let path = decodeURIComponent(url.pathname);
  if (path === "/") path = "/index.html";
  const file = normalize(join(process.cwd(), ROOT, path));
  try {
    const data = await readFile(file);
    res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    res.end(data);
  } catch {
    res.writeHead(404, { "content-type": "text/plain" }).end("not found");
  }
}).listen(PORT, "127.0.0.1", () => {
  console.log(`serving ${ROOT}/ on http://127.0.0.1:${PORT}`);
});
