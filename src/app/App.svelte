<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import TitleBar from "./TitleBar.svelte";
  import Sidebar from "./Sidebar.svelte";
  import SettingsDialog from "./SettingsDialog.svelte";
  import Toast from "../components/Toast.svelte";
  import TodoDialog from "../features/todos/TodoDialog.svelte";
  import TodoListPanel from "../features/todos/TodoListPanel.svelte";
  import TodoDetailPanel from "../features/todos/TodoDetailPanel.svelte";
  import NoteListPanel from "../features/notes/NoteListPanel.svelte";
  import NoteEditorPanel from "../features/notes/NoteEditorPanel.svelte";
  import CompactView from "../features/compact/CompactView.svelte";
  import { appState } from "../stores/app.svelte";
  import { loadTodos } from "../stores/todos.svelte";
  import { createNote, loadNotes } from "../stores/notes.svelte";
  import { toast } from "../stores/toast.svelte";

  let unlisteners: Array<() => void> = [];

  onMount(() => {
    void loadTodos();
    void loadNotes();

    const reg = <T,>(event: string, handler: (payload: T) => void) => {
      listen<T>(event, (e) => handler(e.payload)).then((u) => unlisteners.push(u));
    };

    reg<{ compact: boolean }>("mode-changed", (p) => {
      appState.mode = p.compact ? "compact" : "normal";
    });
    reg("create-todo", () => {
      appState.tab = "todo";
      appState.todoDialogOpen = true;
    });
    reg<{ ok: boolean; message: string }>("shortcut-status", (p) => {
      if (!p.ok && !appState.shortcutWarningShown) {
        appState.shortcutWarningShown = true;
        toast(`${p.message}；仍可通过系统托盘打开窗口`, "error");
      }
    });
    reg<{ message: string }>("app-error", (p) => toast(p.message, "error"));

    return () => {
      unlisteners.forEach((u) => u());
      unlisteners = [];
    };
  });

  // 搜索词变化：防抖刷新当前页列表
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    void appState.searchQuery;
    void appState.tab;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      if (appState.tab === "todo") void loadTodos();
      else void loadNotes();
    }, 220);
  });

  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      e.preventDefault();
      if (appState.tab === "todo") appState.todoDialogOpen = true;
      else void createNote({});
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if appState.mode === "compact"}
  <CompactView />
{:else}
  <div class="app-shell">
    <Sidebar />
    <div class="content">
      <TitleBar />
      <div class="panels">
        {#if appState.tab === "todo"}
          <TodoListPanel />
          <TodoDetailPanel />
        {:else}
          <NoteListPanel />
          <NoteEditorPanel />
        {/if}
      </div>
    </div>
  </div>
{/if}

<SettingsDialog />
<TodoDialog />
<Toast />
