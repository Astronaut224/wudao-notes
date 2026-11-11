/** 小工具：防抖、日期显示 */

export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number) {
  let t: ReturnType<typeof setTimeout> | null = null;
  const wrapped = (...args: A) => {
    if (t) clearTimeout(t);
    t = setTimeout(() => {
      t = null;
      fn(...args);
    }, ms);
  };
  wrapped.flush = (...args: A) => {
    if (t) {
      clearTimeout(t);
      t = null;
      fn(...args);
    }
  };
  wrapped.cancel = () => {
    if (t) {
      clearTimeout(t);
      t = null;
    }
  };
  return wrapped;
}

/** 列表中的截止日提示：MM-DD 截止 / 今日截止 / 已逾期 */
export function dueLabel(due: string | null): { text: string; cls: "" | "today" | "overdue" } {
  if (!due) return { text: "", cls: "" };
  const today = new Date();
  const todayStr = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  if (due === todayStr) return { text: "今日截止", cls: "today" };
  if (due < todayStr) return { text: "已逾期", cls: "overdue" };
  const [, m, d] = due.split("-");
  return { text: `${m}-${d} 截止`, cls: "" };
}

/** 相对时间：今天 14:30 / 11-09 / 2025-12-01 */
export function timeLabel(ts: string): string {
  if (!ts) return "";
  const d = new Date(ts.replace(" ", "T"));
  if (Number.isNaN(d.getTime())) return ts;
  const now = new Date();
  const sameDay = d.toDateString() === now.toDateString();
  if (sameDay) {
    return `今天 ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }
  const y = d.getFullYear();
  const md = `${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  return y === now.getFullYear() ? md : `${y}-${md}`;
}

/** 文件大小展示 */
export function sizeLabel(bytes: number | null): string {
  if (bytes == null) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/** 文件类型副标题 */
export function kindSublabel(name: string, kind: string): string {
  if (kind === "url") return "网页链接";
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    doc: "Word 文档",
    docx: "Word 文档",
    xls: "Excel 表格",
    xlsx: "Excel 表格",
    ppt: "演示文稿",
    pptx: "演示文稿",
    pdf: "PDF 文档",
    txt: "文本文件",
    md: "Markdown",
    zip: "压缩包",
    "7z": "压缩包",
    rar: "压缩包",
    png: "图片",
    jpg: "图片",
    jpeg: "图片",
    gif: "图片",
    webp: "图片",
    bmp: "图片",
  };
  return map[ext] ?? (kind === "image" ? "图片" : "文件");
}
