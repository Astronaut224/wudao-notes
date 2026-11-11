<script lang="ts">
  /** 二次确认弹窗：标题 + 说明 + 危险主按钮 */
  interface Props {
    open: boolean;
    title: string;
    message: string;
    confirmText?: string;
    danger?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }
  let { open, title, message, confirmText = "确定", danger = false, onConfirm, onCancel }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter") onConfirm();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div
    class="overlay"
    role="dialog"
    aria-modal="true"
    onclick={(e) => e.target === e.currentTarget && onCancel()}
  >
    <div class="dialog">
      <h3>{title}</h3>
      <p class="confirm-message">{message}</p>
      <div class="dialog-row">
        <button class="btn-secondary" onclick={onCancel}>取消</button>
        <button
          class="btn-primary"
          style={danger ? "background:var(--danger)" : ""}
          onclick={onConfirm}
        >
          {confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-message {
    font-size: 12px;
    line-height: 1.9;
    color: var(--text-2);
    margin: 0;
    white-space: pre-line;
  }
</style>
