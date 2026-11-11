/**
 * 轻量 Markdown 渲染器（零依赖，安全优先）。
 * 原则：先对源文本整体 HTML 转义，再生成受控标签 —— 原始 HTML 一律按纯文本显示，
 * 链接仅接受 http(s)，图片/脚本/内联样式/事件属性均不可能注入。
 *
 * 支持：# 标题（1-6 级）、**粗体**、*斜体*、`行内代码`、[文本](url)、
 *       无序/有序列表（支持两档缩进）、> 引用、``` 代码块、--- 分隔线、段落与换行。
 */

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function safeUrl(url: string): string | null {
  const u = url.trim();
  if (/^https?:\/\//i.test(u)) return u;
  return null; // 拒绝 javascript: / file: / data: 等协议
}

/** 行内格式（输入应为已转义文本） */
export function renderInline(escaped: string): string {
  let s = escaped;
  // 行内代码：先摘出，避免内部再被其他规则改写
  const codes: string[] = [];
  s = s.replace(/`([^`]+)`/g, (_m, c: string) => {
    codes.push(c);
    return `\u0000CODE${codes.length - 1}\u0000`;
  });
  // 链接 [text](url) —— text/url 均已转义
  s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (m, text: string, url: string) => {
    const real = safeUrl(url.replace(/&amp;/g, "&"));
    if (!real) return m;
    const safeAttr = escapeHtml(real);
    return `<a href="${safeAttr}" target="_blank" rel="noopener noreferrer">${text}</a>`;
  });
  // 粗体 / 斜体
  s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/(^|[^*])\*([^*\n]+)\*/g, "$1<em>$2</em>");
  // 还原行内代码
  s = s.replace(/\u0000CODE(\d+)\u0000/g, (_m, i: string) => `<code>${codes[Number(i)]}</code>`);
  return s;
}

export function renderMarkdown(src: string): string {
  const lines = escapeHtml(src.replace(/\r\n/g, "\n")).split("\n");
  const out: string[] = [];

  type ListKind = "ul" | "ol";
  let listStack: ListKind[] = [];

  const closeLists = (toDepth = 0) => {
    while (listStack.length > toDepth) {
      out.push(`</${listStack.pop()}>`);
    }
  };
  const inList = () => listStack.length > 0;

  let inCode = false;
  let codeLines: string[] = [];
  let para: string[] = [];

  const flushPara = () => {
    if (para.length) {
      out.push(`<p>${para.map(renderInline).join("<br>")}</p>`);
      para = [];
    }
  };

  for (const rawLine of lines) {
    const line = rawLine;

    // 围栏代码块
    const fence = line.match(/^\s*```\s*(\S*)\s*$/);
    if (fence) {
      if (inCode) {
        out.push(`<pre><code>${codeLines.join("\n")}</code></pre>`);
        codeLines = [];
        inCode = false;
      } else {
        flushPara();
        closeLists();
        inCode = true;
      }
      continue;
    }
    if (inCode) {
      codeLines.push(line);
      continue;
    }

    // 空行：结束段落
    if (line.trim() === "") {
      flushPara();
      closeLists();
      continue;
    }

    // 标题
    const h = line.match(/^(#{1,6})\s+(.*)$/);
    if (h) {
      flushPara();
      closeLists();
      const level = h[1].length;
      out.push(`<h${level}>${renderInline(h[2].trim())}</h${level}>`);
      continue;
    }

    // 分隔线
    if (/^\s*(-{3,}|\*{3,})\s*$/.test(line)) {
      flushPara();
      closeLists();
      out.push("<hr>");
      continue;
    }

    // 引用
    const quote = line.match(/^\s*&gt;\s?(.*)$/);
    if (quote) {
      flushPara();
      closeLists();
      out.push(`<blockquote>${renderInline(quote[1])}</blockquote>`);
      continue;
    }

    // 列表（支持一档缩进嵌套）
    const ul = line.match(/^(\s*)[-*]\s+(.*)$/);
    const ol = line.match(/^(\s*)\d+[.、)]\s+(.*)$/);
    if (ul || ol) {
      flushPara();
      const indent = (ul ?? ol)![1].length;
      const kind: ListKind = ul ? "ul" : "ol";
      const depth = indent >= 2 ? 2 : 1;
      const target = depth === 2 ? 2 : 1;
      if (!inList()) {
        listStack.push(kind);
        out.push(`<${kind}>`);
      } else if (listStack.length < target) {
        listStack.push(kind);
        out.push(`<${kind}>`);
      } else {
        closeLists(target);
        if (listStack[listStack.length - 1] !== kind) {
          out.push(`</${listStack.pop()}>`);
          listStack.push(kind);
          out.push(`<${kind}>`);
        }
      }
      out.push(`<li>${renderInline((ul ?? ol)![2])}</li>`);
      continue;
    }

    // 普通文本行
    if (inList()) {
      // 列表内的续行并入上一个 li
      const last = out.length - 1;
      out[last] = out[last].replace(/<\/li>$/, `${renderInline(line)}</li>`);
    } else {
      para.push(line);
    }
  }

  if (inCode && codeLines.length) {
    out.push(`<pre><code>${codeLines.join("\n")}</code></pre>`);
  }
  flushPara();
  closeLists();
  return out.join("\n");
}
