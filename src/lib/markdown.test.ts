import { describe, expect, it } from "vitest";
import { escapeHtml, renderInline, renderMarkdown } from "./markdown";

describe("escapeHtml", () => {
  it("转义所有危险字符", () => {
    expect(escapeHtml(`<script>alert("x")</script>`)).toBe(
      "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;",
    );
    expect(escapeHtml(`'&`)).toBe("&#39;&amp;");
  });
});

describe("renderInline", () => {
  it("输入已是转义文本，输出不产生新危险", () => {
    const out = renderInline(escapeHtml('<b onmouseover="x">bold</b> **真实粗体**'));
    expect(out).toContain("<strong>真实粗体</strong>");
    expect(out).not.toContain("<b ");
  });

  it("支持粗体/斜体/行内代码", () => {
    expect(renderInline("**粗** *斜* `代码`")).toBe(
      "<strong>粗</strong> <em>斜</em> <code>代码</code>",
    );
  });

  it("链接仅允许 http(s)", () => {
    expect(renderInline(escapeHtml("[x](https://a.com)"))).toContain('href="https://a.com"');
    expect(renderInline(escapeHtml("[x](javascript:alert(1))"))).not.toContain('href="javascript');
    expect(renderInline(escapeHtml("[x](file://c:/windows)"))).not.toContain('href="file');
  });
});

describe("renderMarkdown", () => {
  it("标题与段落", () => {
    const html = renderMarkdown("# 标题\n\n正文一行");
    expect(html).toContain("<h1>标题</h1>");
    expect(html).toContain("<p>正文一行</p>");
  });

  it("代码块整体转义", () => {
    const html = renderMarkdown("```\n<script>x</script>\n```");
    expect(html).toContain("<pre><code>");
    expect(html).not.toContain("<script>");
  });

  it("无序/有序列表", () => {
    const html = renderMarkdown("- 甲\n- 乙\n\n1. 一\n2. 二");
    expect(html).toContain("<ul>");
    expect(html).toContain("<li>甲</li>");
    expect(html).toContain("<ol>");
    expect(html).toContain("<li>二</li>");
  });

  it("引用与分隔线", () => {
    const html = renderMarkdown("> 引用内容\n\n---");
    expect(html).toContain("<blockquote>引用内容</blockquote>");
    expect(html).toContain("<hr>");
  });

  it("链接可点击且带 noopener", () => {
    const html = renderMarkdown("[文档](https://example.com/doc?a=1)");
    expect(html).toContain('<a href="https://example.com/doc?a=1" target="_blank" rel="noopener noreferrer">文档</a>');
  });

  it("原始 HTML 注入被显示为文本", () => {
    const html = renderMarkdown('<img src=x onerror=alert(1)>**普通**');
    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;img");
  });

  it("单行换行转为 <br>", () => {
    const html = renderMarkdown("第一行\n第二行");
    expect(html).toContain("第一行<br>第二行");
  });
});
