<script lang="ts">
  /** 新建待办弹框：默认仅标题 + 优先级，其余字段折叠（快捷输入原则） */
  import { appState } from "../../stores/app.svelte";
  import { createTodo } from "../../stores/todos.svelte";
  import type { Priority } from "../../lib/types";

  let title = $state("");
  let priority = $state<Priority>(2);
  let expanded = $state(false);
  let nextAction = $state("");
  let dueDate = $state("");
  let saving = $state(false);
  let titleInput: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (appState.todoDialogOpen) {
      title = "";
      priority = 2;
      expanded = false;
      nextAction = "";
      dueDate = "";
      setTimeout(() => titleInput?.focus(), 60);
    }
  });

  async function save() {
    if (!title.trim() || saving) return;
    saving = true;
    try {
      await createTodo({
        title: title.trim(),
        priority,
        next_action: expanded ? nextAction.trim() : undefined,
        due_date: expanded && dueDate ? dueDate : undefined,
      });
      appState.todoDialogOpen = false;
    } finally {
      saving = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (!appState.todoDialogOpen) return;
    if (e.key === "Escape") appState.todoDialogOpen = false;
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void save();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if appState.todoDialogOpen}
  <div class="overlay" role="dialog" aria-modal="true" onclick={(e) => e.target === e.currentTarget && (appState.todoDialogOpen = false)}>
    <div class="dialog">
      <h3>新建待办</h3>
      <label for="td-title">标题</label>
      <input
        id="td-title"
        class="edit-input"
        bind:this={titleInput}
        bind:value={title}
        placeholder="例如：确认客户的最新修改意见"
        maxlength="160"
        spellcheck="false"
      />
      <label for="td-priority">优先级</label>
      <select id="td-priority" class="edit-input" bind:value={priority}>
        <option value={2}>中优先（默认）</option>
        <option value={1}>高优先</option>
        <option value={3}>低优先</option>
      </select>

      <button class="expand-toggle" onclick={() => (expanded = !expanded)}>
        {expanded ? "收起更多选项 ▲" : "填写下一步 / 截止日期 ▼"}
      </button>
      {#if expanded}
        <label for="td-next">下一步（可选）</label>
        <input id="td-next" class="edit-input" bind:value={nextAction} placeholder="一句具体动作" maxlength="500" spellcheck="false" />
        <label for="td-due">截止日期（可选）</label>
        <input id="td-due" class="edit-input" type="date" bind:value={dueDate} />
      {/if}

      <div class="dialog-row">
        <button class="btn-secondary" onclick={() => (appState.todoDialogOpen = false)}>取消</button>
        <button class="btn-primary" disabled={!title.trim() || saving} onclick={() => void save()}>
          保存
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .expand-toggle {
    border: 0;
    background: transparent;
    color: #5071ed;
    font-size: 11px;
    padding: 10px 0 0;
  }
</style>
