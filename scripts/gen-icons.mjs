// 生成 Tauri 所需的图标（无第三方依赖）。
// 产出：src-tauri/icons/{32x32.png,128x128.png,128x128@2x.png,icon.ico}
// 图案：左半暖色（日）、右半冷色（夜）的圆，带抗锯齿。

import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "src-tauri", "icons");
mkdirSync(outDir, { recursive: true });

/** 返回该像素的 RGBA（描边抗锯齿） */
function pixel(x, y, size) {
  const c = size / 2;
  const r = size / 2 - size * 0.03;
  const dx = x + 0.5 - c;
  const dy = y + 0.5 - c;
  const dist = Math.hypot(dx, dy);
  const a = Math.max(0, Math.min(1, r + 0.5 - dist));
  if (a <= 0) return [0, 0, 0, 0];
  // 左半暖色 #FFBA42，右半冷色 #2B344A
  const [cr, cg, cb] = x + 0.5 < c ? [255, 186, 66] : [43, 52, 74];
  return [cr, cg, cb, Math.round(a * 255)];
}

// ---------------------------------------------------------------- PNG

const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

function png(size) {
  const raw = Buffer.alloc(size * (size * 4 + 1));
  let p = 0;
  for (let y = 0; y < size; y++) {
    raw[p++] = 0; // filter: none
    for (let x = 0; x < size; x++) {
      const [r, g, b, a] = pixel(x, y, size);
      raw[p++] = r;
      raw[p++] = g;
      raw[p++] = b;
      raw[p++] = a;
    }
  }

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---------------------------------------------------------------- ICO（BMP 条目）

function icoEntry(size) {
  const xor = Buffer.alloc(size * size * 4);
  const maskRow = Math.ceil(size / 32) * 4;
  const mask = Buffer.alloc(maskRow * size);
  let p = 0;
  // BMP 自下而上
  for (let y = size - 1; y >= 0; y--) {
    for (let x = 0; x < size; x++) {
      const [r, g, b, a] = pixel(x, y, size);
      xor[p++] = b;
      xor[p++] = g;
      xor[p++] = r;
      xor[p++] = a;
    }
  }

  const hdr = Buffer.alloc(40);
  hdr.writeUInt32LE(40, 0);
  hdr.writeInt32LE(size, 4);
  hdr.writeInt32LE(size * 2, 8); // XOR + AND
  hdr.writeUInt16LE(1, 12);
  hdr.writeUInt16LE(32, 14);
  hdr.writeUInt32LE(0, 16);
  hdr.writeUInt32LE(xor.length + mask.length, 20);

  return Buffer.concat([hdr, xor, mask]);
}

function ico(sizes) {
  const entries = sizes.map((s) => icoEntry(s));
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(sizes.length, 4);

  let offset = 6 + 16 * sizes.length;
  const dir = Buffer.alloc(16 * sizes.length);
  sizes.forEach((s, i) => {
    const b = 16 * i;
    dir[b] = s >= 256 ? 0 : s;
    dir[b + 1] = s >= 256 ? 0 : s;
    dir[b + 2] = 0;
    dir[b + 3] = 0;
    dir.writeUInt16LE(1, b + 4);
    dir.writeUInt16LE(32, b + 6);
    dir.writeUInt32LE(entries[i].length, b + 8);
    dir.writeUInt32LE(offset, b + 12);
    offset += entries[i].length;
  });

  return Buffer.concat([header, dir, ...entries]);
}

// ---------------------------------------------------------------- 写出

writeFileSync(join(outDir, "32x32.png"), png(32));
writeFileSync(join(outDir, "128x128.png"), png(128));
writeFileSync(join(outDir, "128x128@2x.png"), png(256));
writeFileSync(join(outDir, "icon.png"), png(512));
writeFileSync(join(outDir, "icon.ico"), ico([16, 32, 48, 64]));

console.log("icons ->", outDir);
