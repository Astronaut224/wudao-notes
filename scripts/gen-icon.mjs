// 生成应用图标源图（1024×1024 PNG，纯 Node 无依赖）。
// 设计：主色蓝圆角方块 + 三条白色横线（笔记/清单意象）。
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SIZE = 1024;
const px = new Uint8Array(SIZE * SIZE * 4);

const cTop = [0x58, 0x83, 0xff];
const cBottom = [0x36, 0x55, 0xd8];
const R = 208; // 圆角半径
const R2 = R * R;

function insideRoundedRect(x, y, x0, y0, x1, y1, r) {
  if (x < x0 || x > x1 || y < y0 || y > y1) return false;
  const cx = x < x0 + r ? x0 + r : x > x1 - r ? x1 - r : x;
  const cy = y < y0 + r ? y0 + r : y > y1 - r ? y1 - r : y;
  const dx = x - cx, dy = y - cy;
  return dx * dx + dy * dy <= r * r || (x >= x0 + r && x <= x1 - r) || (y >= y0 + r && y <= y1 - r);
}

function blend(i, r, g, b, a) {
  const na = a / 255, oa = px[i + 3] / 255;
  const outA = na + oa * (1 - na);
  if (outA <= 0) return;
  px[i] = Math.round((r * na + px[i] * oa * (1 - na)) / outA);
  px[i + 1] = Math.round((g * na + px[i + 1] * oa * (1 - na)) / outA);
  px[i + 2] = Math.round((b * na + px[i + 2] * oa * (1 - na)) / outA);
  px[i + 3] = Math.round(outA * 255);
}

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const i = (y * SIZE + x) * 4;
    // 圆角方块背景（对角渐变）
    if (insideRoundedRect(x, y, 32, 32, SIZE - 33, SIZE - 33, R)) {
      const t = (x + y) / (2 * SIZE);
      const r = cTop[0] + (cBottom[0] - cTop[0]) * t;
      const g = cTop[1] + (cBottom[1] - cTop[1]) * t;
      const b = cTop[2] + (cBottom[2] - cTop[2]) * t;
      blend(i, r, g, b, 255);
    }
  }
}

// 三条白色圆角横线（清单意象，长短错落）
const bars = [
  { x0: 272, y0: 300, x1: 752, y1: 372 },
  { x0: 272, y0: 476, x1: 656, y1: 548 },
  { x0: 272, y0: 652, x1: 752, y1: 724 },
];
for (const b of bars) {
  for (let y = Math.floor(b.y0); y <= Math.ceil(b.y1); y++) {
    for (let x = Math.floor(b.x0); x <= Math.ceil(b.x1); x++) {
      const i = (y * SIZE + x) * 4;
      // 抗锯齿：到圆角矩形的有符号距离
      const cx = x < b.x0 + 36 ? b.x0 + 36 : x > b.x1 - 36 ? b.x1 - 36 : x;
      const cy = y < b.y0 + 36 ? b.y0 + 36 : y > b.y1 - 36 ? b.y1 - 36 : y;
      const dist = Math.hypot(x - cx, y - cy) - 36;
      const a = Math.max(0, Math.min(255, Math.round((0.5 - dist) * 255)));
      if (a > 0) blend(i, 255, 255, 255, a);
    }
  }
}

// PNG 编码
function crc32(buf) {
  let c, table = [];
  for (let n = 0; n < 256; n++) {
    c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  let crc = 0xffffffff;
  for (const byte of buf) crc = table[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0; // filter: none
  Buffer.from(px.buffer, y * SIZE * 4, SIZE * 4).copy(raw, y * (SIZE * 4 + 1) + 1);
}
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const outDir = join(dirname(fileURLToPath(import.meta.url)));
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, "icon-source.png"), png);
console.log("icon-source.png written:", png.length, "bytes");
