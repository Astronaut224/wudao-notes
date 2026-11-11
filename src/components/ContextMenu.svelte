<script lang="ts">
  /** 右键菜单：由父组件传入定位与条目，选择后回调 */
  export interface CtxItem {
    key: string;
    label: string;
    danger?: boolean;
    disabled?: boolean;
    separatorBefore?: boolean;
  }
  interface Props {
    x: number;
    y: number;
    items: CtxItem[];
    onSelect: (key: string) => void;
    onClose: () => void;
  }
  let { x, y, items, onSelect, onClose }: Props = $props();

  function onGlobalClick() {
    onClose();
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
  // 定位防溢出
  const posStyle = $derived.by(() => {
    const menuW = 168;
    const menuH = items.length * 34 + 14;
    const px = Math.max(4, Math.min(x, window.innerWidth - menuW));
    const py = Math.max(4, Math.min(y, window.innerHeight - menuH));
    return `left:${px}px;top:${py}px`;
  });
</script>

<svelte:window onclick={onGlobalClick} onkeydown={onKeydown} oncontextmenu={onGlobalClick} />

<div class="ctx-menu" style={posStyle} onclick={(e) => e.stopPropagation()}>
  {#each items as it (it.key)}
    {#if it.separatorBefore}
      <div class="ctx-sep"></div>
    {/if}
    <button
      class={it.danger ? "danger" : ""}
      disabled={it.disabled}
      onclick={() => {
        onSelect(it.key);
        onClose();
      }}
    >
      {it.label}
    </button>
  {/each}
</div>
