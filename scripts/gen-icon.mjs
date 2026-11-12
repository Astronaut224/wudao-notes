// 生成应用图标源图（1024×1024 PNG，纯 Node 无依赖）。
// 设计：主色蓝圆角方块 + 白色羽毛笔（quill）线条图案。
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SIZE = 1024;
const px = new Uint8Array(SIZE * SIZE * 4);

const cTop = [0x58, 0x83, 0xff];
const cBottom = [0x36, 0x55, 0xd8];

function blend(i, r, g, b, a) {
  const na = a / 255, oa = px[i + 3] / 255;
  const outA = na + oa * (1 - na);
  if (outA <= 0) return;
  px[i] = Math.round((r * na + px[i] * oa * (1 - na)) / outA);
  px[i + 1] = Math.round((g * na + px[i + 1] * oa * (1 - na)) / outA);
  px[i + 2] = Math.round((b * na + px[i + 2] * oa * (1 - na)) / outA);
  px[i + 3] = Math.round(outA * 255);
}

// 圆角方块背景（对角渐变）
(function background() {
  const R = 208, R2 = R * R;
  for (let y = 0; y < SIZE; y++) {
    for (let x = 0; x < SIZE; x++) {
      if (x < 32 || x > SIZE - 33 || y < 32 || y > SIZE - 33) continue;
      // 圆角判定：到四角圆心的距离
      const cx = x < 32 + R ? 32 + R : x > SIZE - 33 - R ? SIZE - 33 - R : x;
      const cy = y < 32 + R ? 32 + R : y > SIZE - 33 - R ? SIZE - 33 - R : y;
      const dx = x - cx, dy = y - cy;
      const inCornerZone = (x < 32 + R || x > SIZE - 33 - R) && (y < 32 + R || y > SIZE - 33 - R);
      if (inCornerZone && dx * dx + dy * dy > R2) continue;
      const i = (y * SIZE + x) * 4;
      const t = (x + y) / (2 * SIZE);
      blend(
        i,
        cTop[0] + (cBottom[0] - cTop[0]) * t,
        cTop[1] + (cBottom[1] - cTop[1]) * t,
        cTop[2] + (cBottom[2] - cTop[2]) * t,
        255,
      );
    }
  }
})();

// ---------- 羽毛笔（Lucide feather 造型，24 视窗映射到 640px 居中） ----------
const S = 26.67; // 24 → 640
const OX = (SIZE - 24 * S) / 2;
const OY = OX;
const toPx = (v) => OX + v * S;

function distToSegment(px_, py_, ax, ay, bx, by) {
  const abx = bx - ax, aby = by - ay;
  const len2 = abx * abx + aby * aby;
  let t = len2 === 0 ? 0 : ((px_ - ax) * abx + (py_ - ay) * aby) / len2;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(px_ - (ax + abx * t), py_ - (ay + aby * t));
}

// 羽毛主体轮廓：起点 A(20.24,12.24) → 半圆弧到 B(11.75,3.75)（圆心(16,8) r=6，
// 经过右上方鼓起）→ 直线到 (5,10.5) → (5,19) → (13.5,19) → 闭合回 A
function bodyPoints() {
  const pts = [];
  const CX = 16, CY = 8, R = 6;
  for (let i = 0; i <= 48; i++) {
    const theta = ((45 - 180 * (i / 48)) * Math.PI) / 180;
    pts.push([CX + R * Math.cos(theta), CY + R * Math.sin(theta)]);
  }
  pts.push([5, 10.5], [5, 19], [13.5, 19], [20.24, 12.24]);
  return pts;
}

const STROKE_W = 2.1; // 24 视窗单位
const half = (STROKE_W * S) / 2;

function strokePolyline(pts, widthHalf) {
  const segs = [];
  for (let i = 0; i < pts.length - 1; i++) segs.push([pts[i], pts[i + 1]]);
  for (const [[ax, ay], [bx, by]] of segs) {
    const pax = toPx(ax), pay = toPx(ay), pbx = toPx(bx), pby = toPx(by);
    const minX = Math.max(0, Math.floor(Math.min(pax, pbx) - widthHalf - 2));
    const maxX = Math.min(SIZE - 1, Math.ceil(Math.max(pax, pbx) + widthHalf + 2));
    const minY = Math.max(0, Math.floor(Math.min(pay, pby) - widthHalf - 2));
    const maxY = Math.min(SIZE - 1, Math.ceil(Math.max(pay, pby) + widthHalf + 2));
    for (let y = minY; y <= maxY; y++) {
      for (let x = minX; x <= maxX; x++) {
        const d = distToSegment(x + 0.5, y + 0.5, pax, pay, pbx, pby);
        const a = Math.max(0, Math.min(255, Math.round((widthHalf + 0.5 - d) * 255)));
        if (a > 0) blend((y * SIZE + x) * 4, 255, 255, 255, a);
      }
    }
  }
}

strokePolyline(bodyPoints(), half);          // 羽毛主体
strokePolyline([[16, 8], [2, 22]], half);    // 羽轴（延伸出笔尖）
strokePolyline([[17.5, 15], [9, 15]], half); // 羽枝线

// ---------- PNG 编码 ----------
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
