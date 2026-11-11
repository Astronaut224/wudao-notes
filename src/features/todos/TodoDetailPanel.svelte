<script lang="ts">
  /** 待办详情：标题/优先级/截止日编辑 + 下一步、备注自动保存 + 资料区 + 关联笔记 */
  import ResourceSection from "../resources/ResourceSection.svelte";
  import { appState } from "../../stores/app.svelte";
  import {
    patchTodo,
    setTodoDone,
    todoStore,
  } from "../../stores/todos.svelte";
  import { createNote, selectNote, noteStore } from "../../stores/notes.svelte";
  import { dueLabel } from "../../lib/utils";
  import type { Priority } from "../../lib/types";

  const d = $derived(todoStore.detail);

  // ---------- 自动保存（下一步 / 备注 / 标题）----------
  let nextActionDraft = $state("");
  let descDraft = $state("");
  let titleDraft = $state("");
  let saveTimers: Record<string, ReturnType<typeof setTimeout> | null> = {};
  let currentId = $state<string | null>(null);

  $effect(() => {
    // 切换选中条目时同步草稿
    const id = todoStore.selectedId;
    currentId = id;
    nextActionDraft = d?.next_action ?? "";
    descDraft = d?.description ?? "";
    titleDraft = d?.title ?? "";
    saveTimers = {};
  });

  function scheduleSave(field: "next_action" | "description" | "title", value: string) {
    if (!currentId) return;
    const t = saveTimers[field];
    if (t) clearTimeout(t);
    saveTimers[field] = setTimeout(() => {
      saveTimers[field] = null;
      void patchTodo(currentId!, { [field]: value });
    }, 650);
  }
  function flushSave(field: "next_action" | "description" | "title") {
    if (!currentId) return;
    const t = saveTimers[field];
    if (t) {
      clearTimeout(t);
      saveTimers[field] = null;
    }
    const value =
      field === "next_action" ? nextActionDraft : field === "description" ? descDraft : titleDraft;
    void patchTodo(currentId, { [field]: value });
  }

  function onPriorityChange(e: Event) {
    const v = Number((e.target as HTMLSelectElement).value) as Priority;
    if (d) void patchTodo(d.id, { priority: v });
  }
  function onDueChange(e: Event) {
    const v = (e.target as HTMLInputElement).value;
    if (d) void patchTodo(d.id, { due_date: v });
  }

  function openNote(noteId: string) {
    appState.tab = "notes";
    // 笔记列表已加载后选中
    setTimeout(() => void selectNote(noteId), 50);
    void noteStore;
  }

  const prText = $derived(
    d ? (d.priority === 1 ? "高优先" : d.priority === 2 ? "中优先" : "低优先") : "",
  );
  const prCls = $derived(d ? (d.priority === 1 ? "high" : d.priority === 2 ? "mid" : "low") : "");
</script>

<section class="panel-detail">
  {#if d}
    <div class="small-label">待办详情</div>
    <div class="detail-head">
      <div style="flex:1; min-width:0">
        <input
          class="d-title-input"
          bind:value={titleDraft}
          oninput={() => scheduleSave("title", titleDraft)}
          onblur={() => flushSave("title")}
          aria-label="待办标题"
          spellcheck="false"
        />
        <div class="small-label save-hint">更改自动保存在本地</div>
      </div>
      <span class="pr-badge {prCls}">{prText}</span>
      <button
        class="btn-secondary"
        title={d.status === "done" ? "恢复为未完成" : "标记完成"}
        onclick={() => void setTodoDone(d.id, d.status !== "done")}
      >
        {d.status === "done" ? "恢复未完成" : "✓ 完成"}
      </button>
    </div>

    <div class="fieldgrid">
      <div class="field">
        <label for="d-priority">优先级</label>
        <select id="d-priority" class="edit-input" value={d.priority} onchange={onPriorityChange}>
          <option value="1">高优先</option>
          <option value="2">中优先</option>
          <option value="3">低优先</option>
        </select>
      </div>
      <div class="field">
        <label for="d-due">截止日期（可选）</label>
        <input id="d-due" class="edit-input" type="date" value={d.due_date ?? ""} onchange={onDueChange} />
      </div>
    </div>
    {#if d.due_date}
      <div class="due-inline {dueLabel(d.due_date).cls}">{dueLabel(d.due_date).text}</div>
    {/if}

    <div class="hr"></div>

    <div class="field">
      <label for="d-next">下一步做什么</label>
      <input
        id="d-next"
        class="edit-input"
        bind:value={nextActionDraft}
        placeholder="写一句下一步的具体动作"
        oninput={() => scheduleSave("next_action", nextActionDraft)}
        onblur={() => flushSave("next_action")}
        spellcheck="false"
      />
    </div>
    <div class="field desc-field">
      <label for="d-desc">补充备注</label>
      <textarea
        id="d-desc"
        class="edit-input"
        bind:value={descDraft}
        placeholder="背景、客户反馈、沟通结论等"
        oninput={() => scheduleSave("description", descDraft)}
        onblur={() => flushSave("description")}
      ></textarea>
    </div>

    <div class="hr"></div>

    <ResourceSection ownerType="todo" ownerId={d.id} />

    <div class="hr"></div>

    <div class="section-head">
      <span class="section-title">关联笔记</span>
      <button class="btn-text" onclick={() => void createNote({ todo_id: d.id })}>＋ 新建笔记</button>
    </div>
    {#if todoStore.detailNotes.length === 0}
      <div class="notes-empty">暂无关联笔记，可新建一篇并自动关联到本条待办。</div>
    {:else}
      {#each todoStore.detailNotes as n (n.id)}
        <button class="pill-note" onclick={() => openNote(n.id)}>
          <span>▤ {n.title}</span>
          <span class="pill-arrow">›</span>
        </button>
      {/each}
    {/if}

    <div class="detail-foot">备注、标题与优先级可直接编辑，停止输入后自动保存。</div>
  {:else}
    <div class="empty-state">点击一个待办，查看下一步和相关资料。</div>
  {/if}
</section>

<style>
  .detail-head {
    display: flex;
    gap: 11px;
    align-items: start;
    margin-top: 6px;
  }
  .d-title-input {
    width: 100%;
    font-size: 18px;
    font-weight: 730;
    color: #293348;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 4px 8px;
    margin-left: -8px;
    background: transparent;
    outline: 0;
  }
  .d-title-input:hover {
    background: #f7f9fd;
  }
  .d-title-input:focus {
    border-color: #8ca3f7;
    background: #fff;
  }
  .save-hint {
    margin: 4px 0 0 0;
  }
  .due-inline {
    font-size: 11px;
    color: var(--text-2);
    margin-top: 8px;
  }
  .due-inline.today {
    color: var(--mid);
    font-weight: 700;
  }
  .due-inline.overdue {
    color: var(--danger);
    font-weight: 700;
  }
  .desc-field {
    margin-top: 16px;
  }
  .notes-empty {
    color: #a3aebf;
    font-size: 12px;
  }
  .pill-note {
    display: flex;
    width: 100%;
    background: #f7f9fd;
    color: #62718d;
    padding: 9px 10px;
    border: 0;
    border-radius: 7px;
    font-size: 11px;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    margin-bottom: 6px;
  }
  .pill-note:hover {
    background: #eef2fb;
  }
  .pill-arrow {
    margin-left: auto;
  }
  .detail-foot {
    color: #aab4c3;
    font-size: 10px;
    margin-top: 20px;
  }
</style>
