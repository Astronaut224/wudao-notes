<script lang="ts">
  /**
   * 简洁模式（340×440 浮动小窗）：浏览/勾选/拖拽排序待办、快速新增待办/笔记、
   * 切换胶囊悬浮、展开完整窗口
   */
  import { onMount } from "svelte";
  import FeatherIcon from "../../components/FeatherIcon.svelte";
  import { api } from "../../lib/api";
  import { appState, switchMode } from "../../stores/app.svelte";
  import { loadMiniTodos, moveTodo, selectTodo, setTodoDone, todoStore } from "../../stores/todos.svelte";
  import { createNote } from "../../stores/notes.svelte";
  import { toast, toastError } from "../../stores/toast.svelte";
  import type { TodoSummary } from "../../lib/types";

  let quickText = $state("");
  let quickMode = $state<"todo" | "note">("todo");
  let onTop = $state(false);

  const active = $derived(
    todoStore.miniItems
      .slice()
      .sort((a, b) => a.priority - b.priority || a.sort_order - b.sort_order)
      .slice(0, 10),
  );
  const activeCount = $derived(todoStore.miniItems.length);

  onMount(() => {
    void loadMiniTodos();
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

  function expandFull() {
    void switchMode("normal");
  }
  function toCapsule() {
    void switchMode("capsule");
  }

  function toggleTop() {
    onTop = !onTop;
    appState.compactOnTop = onTop;
    void api.setAlwaysOnTop(onTop).catch(() => (onTop = !onTop));
  }

  // ---------- 拖拽排序（指针事件驱动）+ 点击跳详情 ----------
  let dragId = $state<string | null>(null);
  let dropHint = $state<{ id: string; where: "above" | "below" } | null>(null);
  let dragMoved = $state(false);
  let suppressClick = false;
  let dragStart = { x: 0, y: 0 };

  function rowAtPoint(x: number, y: number): HTMLElement | null {
    const el = document.elementFromPoint(x, y);
    return el ? (el.closest("[data-row-id]") as HTMLElement | null) : null;
  }

  function onRowPointerDown(e: PointerEvent, t: TodoSummary) {
    if (e.button !== 0) return;
    // 勾选框等交互子控件不作为拖拽起点，否则 setPointerCapture 会吞掉其 click 事件
    if ((e.target as HTMLElement).closest("button, input, a, select, textarea")) return;
    dragId = t.id;
    dragMoved = false;
    dragStart = { x: e.clientX, y: e.clientY };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);

    const move = (ev: PointerEvent) => {
      if (!dragMoved && Math.hypot(ev.clientX - dragStart.x, ev.clientY - dragStart.y) < 5) return;
      dragMoved = true;
      const el = rowAtPoint(ev.clientX, ev.clientY);
      if (!el || el.dataset.rowId === dragId) {
        dropHint = null;
        return;
      }
      const rect = el.getBoundingClientRect();
      dropHint = {
        id: el.dataset.rowId!,
        where: ev.clientY < rect.top + rect.height / 2 ? "above" : "below",
      };
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      const hint = dropHint;
      const id = dragId;
      dropHint = null;
      dragId = null;
      if (!dragMoved || !hint || !id) return;
      suppressClick = true;
      const target = todoStore.miniItems.find((x) => x.id === hint.id);
      if (!target) return;
      const beforeId = hint.where === "above" ? target.id : nextSiblingOf(target);
      void moveTodo(id, target.priority, beforeId);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
  function nextSiblingOf(t: TodoSummary): string | null {
    const same = todoStore.miniItems
      .filter((x) => x.status === "active" && x.priority === t.priority)
      .sort((a, b) => a.sort_order - b.sort_order);
    const idx = same.findIndex((x) => x.id === t.id);
    return idx >= 0 && idx + 1 < same.length ? same[idx + 1].id : null;
  }

  /** 单击待办：选中并切回完整窗口查看详情 */
  async function openDetailInFull(id: string) {
    appState.tab = "todo";
    await selectTodo(id);
    try {
      await switchMode("normal");
    } catch (e) {
      toastError(e, "打开完整窗口失败");
    }
  }
</script>

<div class="compact-wrap">
  <div class="compact-shell">
    <div class="compact-hd" data-tauri-drag-region>
      <span class="hd-title" data-tauri-drag-region>
        <span class="hd-logo"><FeatherIcon size={13} color="#fff" /></span>吾道笔记
      </span>
      <span class="hd-actions">
        <button title={onTop ? "取消置顶" : "置顶"} aria-label={onTop ? "取消置顶" : "置顶"} class:on={onTop} onclick={toggleTop}>
          {onTop ? "★" : "☆"}
        </button>
        <button title="切换为胶囊悬浮" aria-label="切换为胶囊悬浮" onclick={toCapsule}>◉</button>
        <button title="展开完整窗口" aria-label="展开完整窗口" onclick={expandFull}>□</button>
        <button title="隐藏到托盘" aria-label="隐藏到托盘" onclick={() => void api.hideWindow()}>×</button>
      </span>
    </div>

    <div class="compact-count">
      {quickMode === "todo" ? `当前待办 · ${activeCount} 项` : "快速记笔记"}
    </div>

    {#if quickMode === "todo"}
      <div class="compact-list">
        {#each active as t (t.id)}
          <div
            class="compact-row"
            class:dragging={dragId === t.id}
            class:drop-above={dropHint?.id === t.id && dropHint.where === "above"}
            class:drop-below={dropHint?.id === t.id && dropHint.where === "below"}
            role="button"
            tabindex="0"
            title="点击在完整窗口中查看详情，按住拖动排序"
            data-row-id={t.id}
            onpointerdown={(e) => onRowPointerDown(e, t)}
            onclick={() => {
              if (suppressClick) {
                suppressClick = false;
                return;
              }
              void openDetailInFull(t.id);
            }}
            onkeydown={(e) => e.key === "Enter" && void openDetailInFull(t.id)}
          >
            <button
              class="tick"
              title="标记完成"
              aria-label="标记完成"
              onclick={(e) => {
                e.stopPropagation();
                void setTodoDone(t.id, true);
              }}
            ></button>
            <span class="flex1" title={t.next_action || t.title}>{t.title}</span>
            <span class="pr-badge {t.priority === 1 ? 'high' : t.priority === 2 ? 'mid' : 'low'}">
              {t.priority === 1 ? "高" : t.priority === 2 ? "中" : "低"}
            </span>
          </div>
        {:else}
          <div class="compact-empty">没有未完成的待办</div>
        {/each}
        {#if active.length > 0}
          <div class="compact-hint">按住行可拖动排序，跨优先级拖动会改变优先级</div>
        {/if}
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

    <button class="expand-full" onclick={expandFull}>展开完整窗口 →</button>
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }
  .compact-wrap {
    height: 100vh;
    padding: 7px;
    background: transparent;
  }
  .compact-shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #fff;
    border: 1px solid #e5ebf5;
    border-radius: 12px;
    overflow: hidden;
  }
  .compact-hd {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 12px 10px 14px;
    border-bottom: 1px solid #eff2f7;
    font-size: 13px;
    font-weight: 700;
    flex: none;
  }
  .hd-title {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: #293447;
  }
  .hd-logo {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 6px;
    background: linear-gradient(150deg, #5883ff, #3a58d4);
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
    padding: 11px 15px 5px;
    font-weight: 750;
    font-size: 15px;
    flex: none;
  }
  .compact-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 10px 6px;
  }
  .compact-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 5px;
    border-bottom: 1px solid #f1f3f7;
    font-size: 12px;
    color: #4a576a;
    cursor: default;
    border-radius: 6px;
  }
  .compact-row.dragging {
    opacity: 0.45;
  }
  .compact-row.drop-above {
    box-shadow: 0 -2px 0 0 var(--primary);
  }
  .compact-row.drop-below {
    box-shadow: 0 2px 0 0 var(--primary);
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
    padding: 30px 0;
    font-size: 12px;
  }
  .compact-hint {
    color: #b4bfce;
    font-size: 10px;
    text-align: center;
    padding: 8px 0 4px;
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
    padding: 10px 12px;
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
    padding: 7px 0 9px;
  }
  .expand-full:hover {
    color: var(--primary);
  }
</style>
