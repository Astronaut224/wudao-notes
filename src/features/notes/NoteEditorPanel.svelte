<script lang="ts">
  /** 笔记编辑器：标题/分类/正文自动保存，Markdown 预览，关联待办，资料区 */
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import ResourceSection from "../resources/ResourceSection.svelte";
  import { renderMarkdown } from "../../lib/markdown";
  import { patchNote, deleteNote, loadNotes, noteStore } from "../../stores/notes.svelte";
  import { loadTodos, todoStore } from "../../stores/todos.svelte";
  import { timeLabel } from "../../lib/utils";

  const n = $derived(noteStore.detail);

  let titleDraft = $state("");
  let categoryDraft = $state("");
  let bodyDraft = $state("");
  let previewMode = $state(false);
  let currentId = $state<string | null>(null);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let bodyEl: HTMLTextAreaElement | undefined = $state();
  let confirmDeleteOpen = $state(false);

  $effect(() => {
    const id = noteStore.selectedId;
    currentId = id;
    titleDraft = n?.title ?? "";
    categoryDraft = n?.category ?? "";
    bodyDraft = n?.body ?? "";
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
  });

  $effect(() => {
    // 打开编辑器时确保待办下拉有数据（只拉一次列表）
    if (noteStore.selectedId && todoStore.items.length === 0) void loadTodos();
  });

  function scheduleSave() {
    if (!currentId) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = null;
      void commit();
    }, 650);
  }
  function flushSave() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    void commit();
  }
  async function commit() {
    if (!currentId || !n) return;
    const patch: Parameters<typeof patchNote>[1] = {};
    if (titleDraft !== n.title) patch.title = titleDraft;
    if (categoryDraft.trim() !== (n.category ?? "")) patch.category = categoryDraft.trim();
    if (bodyDraft !== n.body) patch.body = bodyDraft;
    if (Object.keys(patch).length === 0) return;
    const updated = await patchNote(currentId, patch);
    if (updated) void loadNotes();
  }

  function onCategoryChange() {
    // 分类输入直接提交（失焦/回车触发）
    flushSave();
  }

  // ---------- Markdown 工具栏插入 ----------
  function wrapSelection(before: string, after = before) {
    if (!bodyEl) return;
    const { selectionStart: s, selectionEnd: e, value } = bodyEl;
    const selected = value.slice(s, e);
    const next = value.slice(0, s) + before + selected + after + value.slice(e);
    bodyDraft = next;
    scheduleSave();
    requestAnimationFrame(() => {
      bodyEl!.focus();
      bodyEl!.setSelectionRange(s + before.length, s + before.length + selected.length);
    });
  }
  function prefixLine(prefix: string) {
    if (!bodyEl) return;
    const { selectionStart: s, value } = bodyEl;
    const lineStart = value.lastIndexOf("\n", s - 1) + 1;
    const next = value.slice(0, lineStart) + prefix + value.slice(lineStart);
    bodyDraft = next;
    scheduleSave();
    requestAnimationFrame(() => {
      bodyEl!.focus();
      bodyEl!.setSelectionRange(s + prefix.length, s + prefix.length);
    });
  }

  function onTodoLinkChange(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    if (!n) return;
    void patchNote(n.id, { todo_id: v || "" }).then(() => {
      void loadNotes();
    });
  }
</script>

<section class="panel-detail">
  {#if n}
    <div class="hrow">
      <span class="small-label">笔记编辑 · {timeLabel(n.updated_at)}</span>
      <span class="editor-actions">
        <button class="btn-text" onclick={() => (previewMode = !previewMode)}>
          {previewMode ? "✎ 编辑" : "▦ 预览"}
        </button>
        <button class="btn-text" onclick={() => void patchNote(n.id, { is_pinned: !n.is_pinned }).then(() => void loadNotes())}>
          {n.is_pinned ? "取消置顶" : "置顶笔记"}
        </button>
        <button class="btn-text archive" onclick={() => void patchNote(n.id, { is_archived: !n.is_archived }).then(() => void loadNotes())}>
          {n.is_archived ? "取消归档" : "归档"}
        </button>
        <button class="btn-text danger" onclick={() => (confirmDeleteOpen = true)}>删除</button>
      </span>
    </div>

    <input
      class="note-title-input"
      bind:value={titleDraft}
      oninput={scheduleSave}
      onblur={flushSave}
      placeholder="未命名笔记"
      maxlength="160"
      aria-label="笔记标题"
      spellcheck="false"
    />
    <div class="cat-row">
      <span class="small-label">分类</span>
      <input
        class="category-input"
        bind:value={categoryDraft}
        oninput={scheduleSave}
        onchange={onCategoryChange}
        onblur={flushSave}
        placeholder="未分类，如 SQL / 会议纪要"
        maxlength="40"
        list="category-suggestions"
        spellcheck="false"
      />
      <datalist id="category-suggestions">
        {#each noteStore.categories as c (c)}
          <option value={c}></option>
        {/each}
      </datalist>
    </div>

    {#if !previewMode}
      <div class="note-toolbar">
        <button title="粗体" onclick={() => wrapSelection("**")}>** 粗体</button>
        <button title="标题" onclick={() => prefixLine("## ")}># 标题</button>
        <button title="列表" onclick={() => prefixLine("- ")}>≡ 列表</button>
        <button title="代码块" onclick={() => wrapSelection("\n```\n", "\n```\n")}>＜/＞ 代码</button>
        <button title="链接" onclick={() => wrapSelection("[", "](https://)")}>↗ 链接</button>
        <span class="toolbar-note">Markdown</span>
      </div>
      <textarea
        bind:this={bodyEl}
        bind:value={bodyDraft}
        class="body-editor"
        oninput={scheduleSave}
        onblur={flushSave}
        placeholder="记录值得留下的内容…"
        spellcheck="false"
      ></textarea>
    {:else}
      <div class="md-preview">{@html renderMarkdown(bodyDraft)}</div>
    {/if}

    <div class="hr"></div>

    <ResourceSection ownerType="note" ownerId={n.id} />

    <div class="hr"></div>

    <div class="section-head">
      <span class="section-title">关联待办（可选）</span>
    </div>
    <select class="edit-input" value={n.todo_id ?? ""} onchange={onTodoLinkChange}>
      <option value="">不关联任何待办</option>
      {#each todoStore.items as t (t.id)}
        <option value={t.id}>{t.status === "done" ? "✓ " : ""}{t.title}</option>
      {/each}
    </select>

    <div class="detail-foot">停止输入后自动保存到本地；预览不执行任何 HTML 或脚本。</div>

    <ConfirmDialog
      open={confirmDeleteOpen}
      title="删除笔记"
      message={`删除「${n.title}」后无法恢复，其关联资料（应用内副本）将一并删除。`}
      confirmText="删除"
      danger
      onConfirm={() => {
        confirmDeleteOpen = false;
        void deleteNote(n.id);
      }}
      onCancel={() => (confirmDeleteOpen = false)}
    />
  {:else}
    <div class="empty-state">选择或新建一篇笔记。</div>
  {/if}
</section>

<style>
  .editor-actions {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .btn-text.danger {
    color: var(--danger);
  }
  .btn-text.archive {
    color: #8a97a9;
  }
  .note-title-input {
    width: 100%;
    font-size: 20px;
    font-weight: 750;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 6px 8px;
    margin: 10px 0 0 -8px;
    color: #293447;
    outline: 0;
    background: transparent;
  }
  .note-title-input:hover {
    background: #f7f9fd;
  }
  .note-title-input:focus {
    border-color: #8ca3f7;
    background: #fff;
  }
  .cat-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
  .category-input {
    border: none;
    background: #eef2ff;
    padding: 6px 11px;
    border-radius: 20px;
    color: #5a71bd;
    font-size: 11px;
    max-width: 220px;
    outline: 0;
  }
  .category-input:focus {
    background: #e4eaff;
  }
  .note-toolbar {
    display: flex;
    gap: 14px;
    align-items: center;
    border-bottom: 1px solid #eef1f7;
    padding: 14px 0 10px;
    margin-top: 12px;
  }
  .note-toolbar button {
    border: 0;
    background: transparent;
    color: #8392a9;
    font-size: 12px;
    padding: 3px 6px;
    border-radius: 5px;
  }
  .note-toolbar button:hover {
    background: #eef2fb;
    color: var(--primary);
  }
  .toolbar-note {
    margin-left: auto;
    color: #a6b2c3;
    font-size: 11px;
  }
  .body-editor {
    width: 100%;
    min-height: 320px;
    border: 0;
    outline: 0;
    padding: 12px 0;
    line-height: 1.9;
    font-size: 13px;
    color: #374258;
    resize: vertical;
    background: transparent;
    user-select: text;
  }
  .md-preview {
    padding: 12px 2px;
    line-height: 1.9;
    font-size: 13px;
    color: #374258;
    min-height: 200px;
    user-select: text;
    overflow-wrap: anywhere;
  }
  .md-preview :global(h1),
  .md-preview :global(h2),
  .md-preview :global(h3),
  .md-preview :global(h4) {
    margin: 14px 0 8px;
    line-height: 1.5;
  }
  .md-preview :global(pre) {
    background: #f4f6fb;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
    overflow-x: auto;
    font-size: 12px;
  }
  .md-preview :global(code) {
    font-family: Consolas, "Courier New", monospace;
    background: #f1f4fa;
    border-radius: 4px;
    padding: 1px 5px;
  }
  .md-preview :global(pre code) {
    background: transparent;
    padding: 0;
  }
  .md-preview :global(blockquote) {
    margin: 8px 0;
    padding: 6px 12px;
    border-left: 3px solid #c9d6f5;
    color: var(--text-2);
    background: #f8faff;
  }
  .md-preview :global(a) {
    color: var(--primary);
  }
  .detail-foot {
    color: #aab4c3;
    font-size: 10px;
    margin-top: 18px;
  }
</style>
