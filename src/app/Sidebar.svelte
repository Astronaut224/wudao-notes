<script lang="ts">
  import FeatherIcon from "../components/FeatherIcon.svelte";
  import { appState } from "../stores/app.svelte";

  function switchTab(tab: "todo" | "notes") {
    appState.tab = tab;
  }
</script>

<aside class="sidebar">
  <div class="sidebar-title">
    <span class="small-logo"><FeatherIcon size={14} color="#fff" /></span>吾道笔记
  </div>
  <button
    class="nav-item"
    class:active={appState.tab === "todo"}
    onclick={() => switchTab("todo")}
  >
    <span class="symbol">☑</span>待办
  </button>
  <button
    class="nav-item"
    class:active={appState.tab === "notes"}
    onclick={() => switchTab("notes")}
  >
    <span class="symbol">▤</span>笔记
  </button>
  <div class="sidebar-bottom">
    <span>● 本地存储</span>
    <button class="settings-btn" title="设置" onclick={() => (appState.settingsOpen = true)}>
      ⚙
    </button>
  </div>
</aside>

<style>
  .settings-btn {
    margin-left: auto;
    border: 0;
    background: transparent;
    font-size: 17px;
    color: #8290aa;
    padding: 4px;
    border-radius: 6px;
  }
  .settings-btn:hover {
    background: #e8edf5;
    color: var(--text);
  }
</style>
