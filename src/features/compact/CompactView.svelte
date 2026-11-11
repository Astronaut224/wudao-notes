<script lang="ts">
  /** 迷你悬浮模式：同一窗口切换形态，仅浏览置顶待办、勾选完成、快速新增待办/笔记 */
  import { onMount } from "svelte";
  import { api } from "../../lib/api";
  import { appState } from "../../stores/app.svelte";
  import { loadTodos, setTodoDone, todoStore } from "../../stores/todos.svelte";
  import { createNote } from "../../stores/notes.svelte";
  import { toast } from "../../stores/toast.svelte";

  let quickText = $state("");
  let quickMode = $state<"todo" | "note">("todo");
  let onTop = $state(false);

  const active = $derived(
    todoStore.items
      .filter((t) => t.status === "active")
      .sort((a, b) => a.priority - b.priority || a.sort_order - b.sort_order)
      .slice(0, 8),
  );
  const activeCount = $derived(todoStore.items.filter((t) => t.status === "active").length);

  onMount(() => {
    void loadTodos();
    void api
      .getSettings()
      .then((s) => {
        onTop = s.compact_on_top === "true";
        appState.compactOnTop = onTop;
      })
      .catch(() => {});
  });

  async function quickAdd() {
    const text = quickText.trim();
    if (!text) return;
    if (quickMode === "todo") {
      try {
        await api.createTodo({ title: text.slice(0, 160), priority: 2 });
        quickText = "";
        await loadTodos();
        toast("已添加待办");
      } catch {
        toast("添加失败", "error");
      }
    } else {
      const created = await createNote({ title: text.slice(0, 160) });
      if (created) {
        quickText = "";
        toast("已保存笔记，可在完整窗口中编辑内容");
      }
    }
  }

  function expand() {
    void api.setCompactMode(false);
  }

  function toggleTop() {
    onTop = !onTop;
    appState.compactOnTop = onTop;
    void api.setAlwaysOnTop(onTop).catch(() => (onTop = !onTop));
  }
</script>

<div class="compact-shell">
  <div class="compact-hd" data-tauri-drag-region>
    <span class="hd-title" data-tauri-drag-region>◆ 吾道笔记</span>
    <span class="hd-actions">
      <button title={onTop ? "取消置顶" : "置顶"} class:on={onTop} onclick={toggleTop}>{onTop ? "★" : "☆"}</button>
      <button title="展开完整窗口" onclick={expand}>□</button>
      <button title="隐藏到托盘" onclick={() => void api.hideWindow()}>×</button>
    </span>
  </div>

  <div class="compact-count">
    {quickMode === "todo" ? `当前待办 · ${activeCount} 项` : "快速记笔记"}
  </div>

  {#if quickMode === "todo"}
    <div class="compact-list">
      {#each active as t (t.id)}
        <div class="compact-row">
          <button
            class="tick"
            title="标记完成"
            onclick={() => void setTodoDone(t.id, true)}
          ></button>
          <span class="flex1" title={t.next_action || t.title}>{t.title}</span>
          <span class="pr-badge {t.priority === 1 ? 'high' : t.priority === 2 ? 'mid' : 'low'}">
            {t.priority === 1 ? "高" : t.priority === 2 ? "中" : "低"}
          </span>
        </div>
      {:else}
        <div class="compact-empty">没有未完成的待办</div>
      {/each}
    </div>
  {:else}
    <div class="compact-note-hint">输入内容将保存为笔记标题，展开后可补充正文与资料。</div>
  {/if}

  <div class="compact-foot">
    <button
      class="mode-toggle"
      title={quickMode === "todo" ? "切换为快速记笔记" : "切换为快速新增待办"}
      onclick={() => (quickMode = quickMode === "todo" ? "note" : "todo")}
    >
      {quickMode === "todo" ? "✎" : "☑"}
    </button>
    <input
      bind:value={quickText}
      placeholder={quickMode === "todo" ? "快速新增待办…" : "快速记笔记…"}
      onkeydown={(e) => e.key === "Enter" && void quickAdd()}
      spellcheck="false"
    />
    <button class="add-btn" title="添加" onclick={() => void quickAdd()}>＋</button>
  </div>

  <button class="expand-full" onclick={expand}>查看更多 → 展开完整窗口</button>
</div>

<style>
  :global(html),
  :global(body) {
    background: #fff;
  }
  .compact-shell {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #fff;
    overflow: hidden;
  }
  .compact-hd {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 13px 14px 12px;
    border-bottom: 1px solid #eff2f7;
    font-size: 13px;
    font-weight: 700;
    flex: none;
  }
  .hd-title {
    color: #293447;
  }
  .hd-actions {
    display: flex;
    gap: 4px;
  }
  .hd-actions button {
    border: 0;
    background: transparent;
    color: #8d9aaf;
    font-size: 13px;
    width: 26px;
    height: 24px;
    border-radius: 6px;
  }
  .hd-actions button:hover {
    background: #eef2f9;
    color: var(--text);
  }
  .hd-actions button.on {
    color: var(--primary);
  }
  .compact-count {
    padding: 13px 17px 6px;
    font-weight: 750;
    font-size: 16px;
    flex: none;
  }
  .compact-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 12px 8px;
  }
  .compact-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 5px;
    border-bottom: 1px solid #f1f3f7;
    font-size: 12px;
    color: #4a576a;
  }
  .compact-row .flex1 {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .compact-empty {
    color: #9ba8bd;
    text-align: center;
    padding: 32px 0;
    font-size: 12px;
  }
  .compact-note-hint {
    flex: 1;
    color: #9ba8bd;
    font-size: 11px;
    padding: 10px 18px;
    line-height: 1.9;
  }
  .compact-foot {
    flex: none;
    padding: 12px 14px;
    background: #fafbfe;
    border-top: 1px solid #eff2f7;
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .compact-foot input {
    flex: 1;
    min-width: 0;
    border: 1px solid #e6ebf5;
    border-radius: 7px;
    padding: 9px 10px;
    font-size: 11px;
    outline: 0;
    user-select: text;
  }
  .compact-foot input:focus {
    border-color: #8ca3f7;
  }
  .mode-toggle {
    border: 1px solid #e6ebf5;
    background: #fff;
    color: #6b778c;
    width: 32px;
    height: 32px;
    border-radius: 7px;
    font-size: 13px;
  }
  .mode-toggle:hover {
    color: var(--primary);
    border-color: #b9c9f5;
  }
  .add-btn {
    padding: 8px 11px;
    background: var(--primary);
    border: 0;
    color: #fff;
    border-radius: 8px;
    font-size: 13px;
  }
  .add-btn:hover {
    background: var(--primary-hover);
  }
  .expand-full {
    flex: none;
    border: 0;
    background: transparent;
    color: #8a97a9;
    font-size: 11px;
    padding: 8px 0 10px;
  }
  .expand-full:hover {
    color: var(--primary);
  }
</style>
