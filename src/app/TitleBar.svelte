<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "../lib/api";
  import { appState, switchMode } from "../stores/app.svelte";

  let maximized = $state(false);
  let searchInput: HTMLInputElement | undefined = $state();

  onMount(() => {
    const win = getCurrentWindow();
    void api.windowMaximized().then((v) => (maximized = v));
    const un = win.onResized(() => {
      void api.windowMaximized().then((v) => (maximized = v));
    });
    return () => {
      void un.then((f) => f());
    };
  });

  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<header class="titlebar" data-tauri-drag-region>
  <div class="title" data-tauri-drag-region>{appState.tab === "todo" ? "待办清单" : "我的笔记"}</div>
  <div class="search-box">
    <span aria-hidden="true">⌕</span>
    <input
      bind:this={searchInput}
      bind:value={appState.searchQuery}
      placeholder="搜索待办和笔记"
      spellcheck="false"
    />
    {#if appState.searchQuery}
      <button
        class="search-clear"
        title="清空搜索"
        onclick={() => (appState.searchQuery = "")}>×</button
      >
    {:else}
      <span class="kbd">Ctrl K</span>
    {/if}
  </div>
  <div class="wincontrols">
    <button title="切换到简洁模式" aria-label="切换到简洁模式" onclick={() => void switchMode("compact")}>▣</button>
    <button title="切换为胶囊悬浮" aria-label="切换为胶囊悬浮" onclick={() => void switchMode("capsule")}>◉</button>
    <button title="最小化" aria-label="最小化窗口" onclick={() => void api.minimizeWindow()}>−</button>
    <button
      title={maximized ? "还原" : "最大化"}
      aria-label={maximized ? "还原窗口" : "最大化窗口"}
      onclick={() => void api.toggleMaximizeWindow()}
    >
      {maximized ? "❐" : "□"}
    </button>
    <button class="close" title="隐藏到托盘" aria-label="隐藏到托盘" onclick={() => void api.hideWindow()}>×</button>
  </div>
</header>

<style>
  .search-clear {
    border: 0;
    background: transparent;
    color: #a8b3c3;
    font-size: 14px;
    padding: 0 2px;
    line-height: 1;
  }
  .search-clear:hover {
    color: var(--text);
  }
</style>
