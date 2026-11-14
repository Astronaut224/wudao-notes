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
  import CapsuleView from "../features/capsule/CapsuleView.svelte";
  import { api } from "../lib/api";
  import { appState } from "../stores/app.svelte";
  import { loadMiniTodos, loadTodos } from "../stores/todos.svelte";
  import { createNote, loadNotes } from "../stores/notes.svelte";
  import { toast } from "../stores/toast.svelte";

  let unlisteners: Array<() => void> = [];

  onMount(() => {
    void loadTodos();
    void loadNotes();

    // 同步启动时的窗口形态（如上一次退出前是简洁/胶囊）
    void api
      .getWindowMode()
      .then((m) => {
        if (m.mode === "compact" || m.mode === "capsule") {
          appState.mode = m.mode;
          appState.capsuleDocked = m.capsuleDocked ?? "";
        }
      })
      .catch(() => {});

    const reg = <T,>(event: string, handler: (payload: T) => void) => {
      listen<T>(event, (e) => handler(e.payload)).then((u) => unlisteners.push(u));
    };

    reg<{ mode: string; compact: boolean }>("mode-changed", (p) => {
      if (p.mode === "normal" || p.mode === "compact" || p.mode === "capsule") {
        appState.mode = p.mode;
      }
    });
    reg<{ docked: boolean; side: string }>("capsule-state", (p) => {
      appState.capsuleDocked = p.docked ? (p.side === "left" ? "left" : "right") : "";
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

  // 进入简洁/胶囊形态时刷新迷你视图数据源（不受完整窗口筛选影响）
  $effect(() => {
    if (appState.mode === "compact" || appState.mode === "capsule") {
      void loadMiniTodos();
    }
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

{#if appState.mode === "capsule"}
  <CapsuleView />
{:else if appState.mode === "compact"}
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
