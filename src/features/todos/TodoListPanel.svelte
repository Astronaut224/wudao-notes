<script lang="ts">
  import ContextMenu, { type CtxItem } from "../../components/ContextMenu.svelte";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import { appState } from "../../stores/app.svelte";
  import { toast, toastError } from "../../stores/toast.svelte";
  import {
    clearCompletedTodos,
    deleteTodo,
    duplicateTodo,
    moveTodo,
    setFilter,
    setTodoDone,
    selectTodo,
    todoStore,
  } from "../../stores/todos.svelte";
  import type { Priority, TodoSummary } from "../../lib/types";
  import { dueLabel } from "../../lib/utils";

  const FILTERS: Array<[string, string]> = [
    ["all", "全部"],
    ["1", "高优先"],
    ["2", "中优先"],
    ["3", "低优先"],
    ["done", "已完成"],
  ];

  interface Group {
    priority: Priority | 0;
    title: string;
    items: TodoSummary[];
  }

  const groups = $derived.by<Group[]>(() => {
    const items = todoStore.items;
    if (todoStore.filter === "done") {
      return [{ priority: 0, title: "已完成", items: items.filter((t) => t.status === "done") }];
    }
    const active = items.filter((t) => t.status === "active");
    const wanted: Priority[] =
      todoStore.filter === "all" ? [1, 2, 3] : [Number(todoStore.filter) as Priority];
    const titles: Record<number, string> = { 1: "高优先", 2: "中优先", 3: "低优先" };
    return wanted.map((p) => ({
      priority: p,
      title: `${titles[p]} · ${active.filter((t) => t.priority === p).length}`,
      items: active.filter((t) => t.priority === p),
    }));
  });

  const activeCount = $derived(todoStore.items.filter((t) => t.status === "active").length);

  // ---------- 拖拽排序（指针事件驱动，不依赖 HTML5 dnd）----------
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
    if (e.button !== 0 || t.status !== "active") return;
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
      const target = todoStore.items.find((x) => x.id === hint.id);
      if (!target) return;
      const beforeId = hint.where === "above" ? target.id : nextSiblingInGroup(target);
      void moveTodo(id, target.priority, beforeId);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
  function nextSiblingInGroup(t: TodoSummary): string | null {
    const same = todoStore.items
      .filter((x) => x.status === "active" && x.priority === t.priority)
      .sort((a, b) => a.sort_order - b.sort_order);
    const idx = same.findIndex((x) => x.id === t.id);
    return idx >= 0 && idx + 1 < same.length ? same[idx + 1].id : null;
  }

  // ---------- 右键菜单 ----------
  let ctx = $state<{ x: number; y: number; todo: TodoSummary } | null>(null);
  let confirmDelete = $state<TodoSummary | null>(null);
  let confirmClearCompleted = $state(false);

  async function doClearCompleted() {
    confirmClearCompleted = false;
    try {
      const r = await clearCompletedTodos();
      toast(
        r.deleted > 0
          ? `已永久删除 ${r.deleted} 条已完成待办${r.detached_notes > 0 ? `，${r.detached_notes} 条关联笔记已解除关联并保留` : ""}`
          : "没有已完成的待办",
      );
    } catch (e) {
      toastError(e, "清空失败");
    }
  }

  function ctxItems(t: TodoSummary): CtxItem[] {
    const done = t.status === "done";
    return [
      { key: "up", label: "上移", disabled: done },
      { key: "down", label: "下移", disabled: done },
      { key: "p1", label: "设为高优先", disabled: done || t.priority === 1, separatorBefore: true },
      { key: "p2", label: "设为中优先", disabled: done || t.priority === 2 },
      { key: "p3", label: "设为低优先", disabled: done || t.priority === 3 },
      { key: "dup", label: "复制条目", separatorBefore: true },
      { key: "del", label: "删除", danger: true },
    ];
  }
  function onCtxSelect(key: string) {
    const t = ctx?.todo;
    ctx = null;
    if (!t) return;
    switch (key) {
      case "up":
      case "down": {
        const same = todoStore.items
          .filter((x) => x.status === "active" && x.priority === t.priority)
          .sort((a, b) => a.sort_order - b.sort_order);
        const i = same.findIndex((x) => x.id === t.id);
        if (key === "up" && i > 0) void moveTodo(t.id, t.priority, same[i - 1].id);
        if (key === "down" && i < same.length - 1) {
          void moveTodo(t.id, t.priority, same[i + 2]?.id ?? null);
        }
        break;
      }
      case "p1":
      case "p2":
      case "p3":
        void moveTodo(t.id, Number(key.slice(1)) as Priority, null);
        break;
      case "dup":
        void duplicateTodo(t.id);
        break;
      case "del":
        confirmDelete = t;
        break;
    }
  }
  function doDelete() {
    const t = confirmDelete;
    confirmDelete = null;
    if (t) void deleteTodo(t.id);
  }
  function deleteMessage(t: TodoSummary): string {
    const notePart =
      t.note_count > 0
        ? `\n· ${t.note_count} 条关联笔记将解除关联并保留`
        : "";
    const resPart =
      t.resource_count > 0
        ? `\n· ${t.resource_count} 条关联资料将一并删除（应用内副本，不影响原文件）`
        : "";
    return `删除「${t.title}」后无法恢复。${notePart}${resPart}`;
  }
</script>

<section class="panel-list">
  <div class="hrow">
    <div>
      <h1 class="heading">我的待办</h1>
      <p class="subtitle">按优先级安排手头的工作，先处理最重要的事</p>
    </div>
    <button class="btn-primary" onclick={() => (appState.todoDialogOpen = true)}>＋ 新建待办</button>
  </div>

  <div class="filters">
    {#each FILTERS as [v, label] (v)}
      <button class="chip" class:active={todoStore.filter === v} onclick={() => setFilter(v as typeof todoStore.filter)}>
        {label}
      </button>
    {/each}
  </div>

  {#each groups as g (g.priority)}
    {#if g.items.length > 0}
      <div class="group-title">
        <span class="group-dot {g.priority === 2 ? 'mid' : g.priority === 3 ? 'low' : ''}"></span>
        {g.priority === 0 ? `已完成 · ${g.items.length}` : g.title}
        {#if g.priority === 0 && g.items.length > 0}
          <button class="clear-completed" onclick={() => (confirmClearCompleted = true)}>
            🗑 永久删除全部
          </button>
        {/if}
      </div>
      {#each g.items as t (t.id)}
        <div
          class="todo-row"
          class:selected={todoStore.selectedId === t.id}
          class:dragging={dragId === t.id}
          class:drop-above={dropHint?.id === t.id && dropHint.where === "above"}
          class:drop-below={dropHint?.id === t.id && dropHint.where === "below"}
          class:done-row={t.status === "done"}
          role="button"
          tabindex="0"
          data-row-id={t.id}
          onpointerdown={(e) => onRowPointerDown(e, t)}
          onclick={() => {
            if (suppressClick) {
              suppressClick = false;
              return;
            }
            void selectTodo(t.id);
          }}
          onkeydown={(e) => e.key === "Enter" && void selectTodo(t.id)}
          oncontextmenu={(e) => {
            e.preventDefault();
            ctx = { x: e.clientX, y: e.clientY, todo: t };
          }}
        >
          <button
            class="tick"
            class:done={t.status === "done"}
            title={t.status === "done" ? "恢复为未完成" : "标记完成"}
            onclick={(e) => {
              e.stopPropagation();
              void setTodoDone(t.id, t.status !== "done");
            }}
          >
            {t.status === "done" ? "✓" : ""}
          </button>
          <div class="todo-main">
            <div class="todo-name" class:done={t.status === "done"}>{t.title}</div>
            {#if t.next_action}
              <div class="todo-next">{t.next_action}</div>
            {/if}
            {#if t.due_date || t.resource_count > 0 || t.note_count > 0}
              <div class="rowmeta">
                {#if t.due_date}
                  <span class="due {dueLabel(t.due_date).cls}">{dueLabel(t.due_date).text}</span>
                {/if}
                {#if t.resource_count > 0}<span title="关联资料">📎 {t.resource_count}</span>{/if}
                {#if t.note_count > 0}<span title="关联笔记">📝 {t.note_count}</span>{/if}
              </div>
            {/if}
          </div>
          <span class="pr-badge {t.priority === 1 ? 'high' : t.priority === 2 ? 'mid' : 'low'}">
            {t.priority === 1 ? "高" : t.priority === 2 ? "中" : "低"}
          </span>
        </div>
      {/each}
    {/if}
  {/each}

  {#if groups.every((g) => g.items.length === 0)}
    <div class="empty-state">
      {#if appState.searchQuery}
        没有匹配「{appState.searchQuery}」的待办。
      {:else if todoStore.filter === "done"}
        还没有已完成的待办。
      {:else}
        这里暂时没有事项，点击「＋ 新建待办」开始吧。
      {/if}
    </div>
  {:else if activeCount === 0 && todoStore.filter !== "done"}
    <div class="empty-state">所有事项都完成了，干得漂亮！</div>
  {/if}
</section>

{#if ctx}
  <ContextMenu x={ctx.x} y={ctx.y} items={ctxItems(ctx.todo)} onSelect={onCtxSelect} onClose={() => (ctx = null)} />
{/if}

<ConfirmDialog
  open={confirmDelete !== null}
  title="删除待办"
  message={confirmDelete ? deleteMessage(confirmDelete) : ""}
  confirmText="删除"
  danger
  onConfirm={doDelete}
  onCancel={() => (confirmDelete = null)}
/>

<ConfirmDialog
  open={confirmClearCompleted}
  title="永久删除已完成待办"
  message={`将永久删除全部 ${todoStore.items.filter((t) => t.status === "done").length} 条已完成待办及其关联资料（应用内副本），关联笔记将解除关联并保留。

此操作不可恢复，确定继续吗？`}
  confirmText="永久删除"
  danger
  onConfirm={() => void doClearCompleted()}
  onCancel={() => (confirmClearCompleted = false)}
/>

<style>
  .clear-completed {
    margin-left: auto;
    border: 0;
    background: transparent;
    color: var(--danger);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 5px;
    letter-spacing: normal;
  }
  .clear-completed:hover {
    background: #fdecea;
  }
  .todo-row {
    border: 1px solid transparent;
    border-radius: 10px;
    padding: 13px 11px 13px 12px;
    display: flex;
    align-items: center;
    gap: 11px;
    margin: 3px -3px;
    cursor: pointer;
    transition: background 0.15s, border 0.15s;
    background: transparent;
  }
  .todo-row:hover {
    background: #f8faff;
  }
  .todo-row.selected {
    background: #f4f7ff;
    border-color: #dfe7ff;
  }
  .todo-main {
    flex: 1;
    min-width: 0;
  }
  .todo-name {
    font-size: 13.4px;
    font-weight: 660;
    color: #293348;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .todo-name.done {
    text-decoration: line-through;
    color: #a5aec0;
  }
  .todo-next {
    font-size: 11px;
    color: #8d9aad;
    margin-top: 5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rowmeta {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 6px;
    color: #9aa7ba;
    font-size: 10px;
  }
  .rowmeta .due.today {
    color: var(--mid);
    font-weight: 700;
  }
  .rowmeta .due.overdue {
    color: var(--danger);
    font-weight: 700;
  }
  .done-row {
    opacity: 0.75;
  }
</style>
