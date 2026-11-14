<script lang="ts">
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import { toast, toastError } from "../../stores/toast.svelte";
  import { appState } from "../../stores/app.svelte";
  import {
    clearArchivedNotes,
    createNote,
    noteStore,
    selectNote,
    setCategoryFilter,
    setShowArchived,
  } from "../../stores/notes.svelte";
  import { timeLabel } from "../../lib/utils";
  import type { NoteSummary } from "../../lib/types";

  const chips = $derived([
    { key: "", label: "全部笔记" },
    ...noteStore.categories.map((c) => ({ key: c, label: c })),
  ]);

  const hasArchived = $derived(noteStore.items.some((n) => n.is_archived));

  function previewOf(n: NoteSummary): string {
    return n.body.replace(/[#*`>|]/g, " ").replace(/\s+/g, " ").trim().slice(0, 76);
  }

  async function onNewNote() {
    await createNote({});
  }

  let creating = $state(false);
  let confirmClearArchived = $state(false);

  async function doClearArchived() {
    confirmClearArchived = false;
    try {
      const n = await clearArchivedNotes();
      toast(n > 0 ? `已永久删除 ${n} 篇已归档笔记` : "没有已归档笔记");
    } catch (e) {
      toastError(e, "清空失败");
    }
  }
</script>

<section class="panel-list">
  <div class="hrow">
    <div>
      <h1 class="heading">我的笔记</h1>
      <p class="subtitle">随手记录，不必归属于某个待办</p>
    </div>
    <button class="btn-primary" disabled={creating} onclick={() => void onNewNote()}>
      ＋ 新建笔记
    </button>
  </div>

  <div class="filters">
    {#each chips as c (c.key)}
      <button
        class="chip"
        class:active={!noteStore.showArchived && noteStore.categoryFilter === c.key}
        onclick={() => setCategoryFilter(c.key)}
      >
        {c.label}
      </button>
    {/each}
    <button class="chip" class:active={noteStore.showArchived} onclick={() => setShowArchived()}>
      已归档
    </button>
  </div>

  {#if noteStore.showArchived && noteStore.items.length > 0}
    <button class="clear-archived" onclick={() => (confirmClearArchived = true)}>
      🗑 永久删除全部已归档笔记（{noteStore.items.length} 篇）
    </button>
  {/if}

  {#each noteStore.items as n (n.id)}
    <div
      class="note-item"
      class:selected={noteStore.selectedId === n.id}
      role="button"
      tabindex="0"
      onclick={() => void selectNote(n.id)}
      onkeydown={(e) => e.key === "Enter" && void selectNote(n.id)}
    >
      <div class="note-item-title">{n.is_pinned ? "📌 " : ""}{n.title}</div>
      {#if previewOf(n)}
        <div class="note-preview">{previewOf(n)}</div>
      {/if}
      <div class="note-time">
        {n.category || "未分类"} · {timeLabel(n.updated_at)}
        {#if n.is_archived}<span class="archived-tag">已归档</span>{/if}
        {#if n.todo_title}<span class="linked-tag">↗ {n.todo_title}</span>{/if}
      </div>
    </div>
  {/each}

  {#if noteStore.items.length === 0}
    <div class="empty-state">
      {#if appState.searchQuery}
        没有匹配「{appState.searchQuery}」的笔记。
      {:else if noteStore.showArchived}
        暂无归档笔记。
      {:else}
        还没有笔记，点击「＋ 新建笔记」随手记一条吧。
      {/if}
    </div>
  {/if}

  {#if hasArchived && !noteStore.showArchived}
    <button class="show-archived-link" onclick={() => setShowArchived()}>查看已归档笔记 →</button>
  {/if}
</section>

<ConfirmDialog
  open={confirmClearArchived}
  title="永久删除已归档笔记"
  message={`将永久删除全部 ${noteStore.items.length} 篇已归档笔记及其关联资料（应用内副本），不可恢复。

确定继续吗？`}
  confirmText="永久删除"
  danger
  onConfirm={() => void doClearArchived()}
  onCancel={() => (confirmClearArchived = false)}
/>

<style>
  .clear-archived {
    display: block;
    width: 100%;
    margin: 10px 0 4px;
    padding: 8px 10px;
    border: 1px solid #f3d9d7;
    background: #fff7f6;
    color: var(--danger);
    border-radius: 8px;
    font-size: 11px;
    text-align: center;
  }
  .clear-archived:hover {
    background: #fdecea;
  }
  .note-item {
    border: 1px solid transparent;
    padding: 14px 13px;
    border-radius: 9px;
    margin: 4px -3px;
    cursor: pointer;
  }
  .note-item:hover {
    background: #f7f9fd;
  }
  .note-item.selected {
    background: #f4f7ff;
    border-color: #dfe7ff;
  }
  .note-item-title {
    font-weight: 700;
    font-size: 13px;
    margin-bottom: 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .note-preview {
    font-size: 11px;
    color: #8e9aac;
    white-space: nowrap;
    text-overflow: ellipsis;
    overflow: hidden;
  }
  .note-time {
    font-size: 10px;
    color: #b1bccc;
    margin-top: 8px;
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .archived-tag,
  .linked-tag {
    background: #f0f3f8;
    border-radius: 4px;
    padding: 1px 6px;
    color: #8794a7;
  }
  .linked-tag {
    background: #eef3ff;
    color: #5372db;
  }
  .show-archived-link {
    display: block;
    margin: 18px auto 0;
    border: 0;
    background: transparent;
    color: #8a97a9;
    font-size: 11px;
  }
  .show-archived-link:hover {
    color: var(--primary);
  }
</style>
