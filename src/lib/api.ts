/** 受控 Tauri 调用封装：前端只允许调用这些命令，不接触 SQL / 文件系统 */
import { invoke } from "@tauri-apps/api/core";
import type {
  Note,
  NotePatch,
  NoteSummary,
  Priority,
  Resource,
  Todo,
  TodoPatch,
  TodoSummary,
} from "./types";

export const api = {
  // ---------- Todo ----------
  listTodos(scope: string, query: string): Promise<TodoSummary[]> {
    return invoke("list_todos", { filter: { scope, query } });
  },
  getTodo(id: string): Promise<Todo> {
    return invoke("get_todo", { id });
  },
  createTodo(input: {
    title: string;
    priority: Priority;
    next_action?: string;
    description?: string;
    due_date?: string;
  }): Promise<Todo> {
    return invoke("create_todo", input);
  },
  updateTodo(id: string, patch: TodoPatch): Promise<Todo> {
    return invoke("update_todo", { id, patch });
  },
  setTodoDone(id: string, done: boolean): Promise<Todo> {
    return invoke("set_todo_done", { id, done });
  },
  reorderTodo(id: string, newPriority: Priority, beforeId: string | null): Promise<Todo> {
    return invoke("reorder_todo", { id, newPriority, beforeId });
  },
  duplicateTodo(id: string): Promise<Todo> {
    return invoke("duplicate_todo", { id });
  },
  deleteTodo(id: string): Promise<{ managed_removed: number; detached_notes: number }> {
    return invoke("delete_todo", { id });
  },
  deleteTodos(ids: string[]): Promise<{ deleted: number; detached_notes: number }> {
    return invoke("delete_todos", { ids });
  },

  // ---------- Note ----------
  /** archived: "unarchived" | "archived" | "all" */
  listNotes(query: string, category: string, archived: string): Promise<NoteSummary[]> {
    return invoke("list_notes", {
      query: query || null,
      category: category || null,
      archived,
    });
  },
  getNote(id: string): Promise<Note> {
    return invoke("get_note", { id });
  },
  createNote(input: {
    title?: string;
    body?: string;
    category?: string;
    todo_id?: string;
  }): Promise<Note> {
    return invoke("create_note", input);
  },
  updateNote(id: string, patch: NotePatch): Promise<Note> {
    return invoke("update_note", { id, patch });
  },
  deleteNote(id: string): Promise<{ managed_removed: number }> {
    return invoke("delete_note", { id });
  },
  clearArchivedNotes(): Promise<{ deleted: number }> {
    return invoke("clear_archived_notes");
  },
  clearCompletedTodos(): Promise<{
    deleted: number;
    detached_notes: number;
    managed_removed: number;
  }> {
    return invoke("clear_completed_todos");
  },
  listNoteCategories(): Promise<string[]> {
    return invoke("list_note_categories");
  },

  // ---------- Resource ----------
  pickFiles(title: string, extensions: string[]): Promise<string[]> {
    return invoke("pick_files", {
      title,
      extensions: extensions.length ? extensions : null,
    });
  },
  pickFile(title: string, extensions: string[]): Promise<string | null> {
    return invoke("pick_file", {
      title,
      extensions: extensions.length ? extensions : null,
    });
  },
  attachLocalFile(
    ownerType: "todo" | "note",
    ownerId: string,
    sourcePath: string,
    mode: "copy" | "reference",
  ): Promise<Resource> {
    return invoke("attach_local_file", { ownerType, ownerId, sourcePath, mode });
  },
  attachUrl(
    ownerType: "todo" | "note",
    ownerId: string,
    displayName: string,
    url: string,
  ): Promise<Resource> {
    return invoke("attach_url", { ownerType, ownerId, displayName, url });
  },
  listResources(ownerType: "todo" | "note", ownerId: string): Promise<Resource[]> {
    return invoke("list_resources", { ownerType, ownerId });
  },
  openResource(id: string): Promise<void> {
    return invoke("open_resource", { id });
  },
  removeResource(id: string): Promise<void> {
    return invoke("remove_resource", { id });
  },
  relocateResource(id: string, newPath: string): Promise<Resource> {
    return invoke("relocate_resource", { id, newPath });
  },

  // ---------- Desktop ----------
  showWindow(): Promise<void> {
    return invoke("show_window");
  },
  hideWindow(): Promise<void> {
    return invoke("hide_window");
  },
  minimizeWindow(): Promise<void> {
    return invoke("minimize_window");
  },
  toggleMaximizeWindow(): Promise<void> {
    return invoke("toggle_maximize_window");
  },
  windowMaximized(): Promise<boolean> {
    return invoke("window_maximized");
  },
  setWindowMode(mode: "normal" | "compact" | "capsule"): Promise<void> {
    return invoke("set_window_mode", { mode });
  },
  getWindowMode(): Promise<{ mode: string; capsuleDocked: string }> {
    return invoke("get_window_mode");
  },
  setCapsuleDocked(docked: boolean, side: string, onlyIfOutside = false): Promise<void> {
    return invoke("set_capsule_docked", { docked, side: side || null, onlyIfOutside });
  },
  moveWindowTo(x: number, y: number): Promise<void> {
    return invoke("move_window_to", { x, y });
  },
  getWindowPosition(): Promise<[number, number]> {
    return invoke("get_window_position");
  },
  getWindowRect(): Promise<[number, number, number, number]> {
    return invoke("get_window_rect");
  },
  getScreenSize(): Promise<[number, number, number, number]> {
    return invoke("get_screen_size");
  },
  setAlwaysOnTop(enabled: boolean): Promise<void> {
    return invoke("set_always_on_top", { enabled });
  },
  setLaunchAtLogin(enabled: boolean): Promise<void> {
    return invoke("set_launch_at_login", { enabled });
  },
  shortcutStatus(): Promise<{ shortcut: string; registered: boolean }> {
    return invoke("shortcut_status");
  },

  // ---------- Settings ----------
  getSettings(): Promise<Record<string, string>> {
    return invoke("get_settings");
  },
  updateSetting(key: string, value: string): Promise<void> {
    return invoke("update_setting", { key, value });
  },
  exportBackup(): Promise<string> {
    return invoke("export_backup");
  },
  prepareRestore(): Promise<{ path: string }> {
    return invoke("prepare_restore");
  },
  executeRestore(path: string): Promise<{ todos: number; notes: number }> {
    return invoke("execute_restore", { path });
  },
};

/** 附件图片内联地址（Rust 自定义协议） */
export function resourceImageUrl(resourceId: string): string {
  return `http://wudaores.localhost/${resourceId}`;
}
