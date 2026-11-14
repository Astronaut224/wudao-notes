/** 待办状态与操作 */
import { api } from "../lib/api";
import type { NoteSummary, Priority, Resource, Todo, TodoSummary } from "../lib/types";
import { errText, type BizErrorShape } from "../lib/types";
import { appState } from "./app.svelte";
import { toast, toastError } from "./toast.svelte";

function scopeOf(filter: string): string {
  if (filter === "all" || filter === "done") return filter;
  return `priority:${filter}`;
}

export const todoStore = $state({
  items: [] as TodoSummary[],
  filter: "all" as "all" | "1" | "2" | "3" | "done",
  selectedId: null as string | null,
  detail: null as Todo | null,
  detailResources: [] as Resource[],
  detailNotes: [] as NoteSummary[],
  /** 已完成视图的多选删除：选中的待办 id */
  selectedDone: [] as string[],
  /** 简洁窗/胶囊专用：全部未完成待办（与完整窗口的筛选无关） */
  miniItems: [] as TodoSummary[],
  listVersion: 0,
});

/** 迷你视图数据源：始终为全部未完成待办 */
export async function loadMiniTodos() {
  try {
    todoStore.miniItems = await api.listTodos("active", "");
  } catch (e) {
    toastError(e, "加载待办失败");
  }
}

export function toggleDoneSelection(id: string) {
  const idx = todoStore.selectedDone.indexOf(id);
  if (idx >= 0) todoStore.selectedDone.splice(idx, 1);
  else todoStore.selectedDone.push(id);
}

export function setAllDoneSelection(selected: boolean) {
  todoStore.selectedDone = selected
    ? todoStore.items.filter((t) => t.status === "done").map((t) => t.id)
    : [];
}

export async function deleteSelectedDone(): Promise<{ deleted: number; detached_notes: number }> {
  const ids = [...todoStore.selectedDone];
  const r = await api.deleteTodos(ids);
  todoStore.selectedDone = [];
  if (todoStore.selectedId && ids.includes(todoStore.selectedId)) {
    await selectTodoSilent(null);
  }
  await loadTodos();
  return { deleted: r.deleted, detached_notes: r.detached_notes };
}

/** 静默清除选中详情（不触发加载） */
async function selectTodoSilent(id: string | null) {
  todoStore.selectedId = id;
  todoStore.detail = null;
  todoStore.detailResources = [];
  todoStore.detailNotes = [];
}

export async function loadTodos() {
  try {
    todoStore.items = await api.listTodos(scopeOf(todoStore.filter), appState.searchQuery);
    // 迷你视图始终显示全部未完成，与完整窗口的筛选/搜索无关
    if (todoStore.filter !== "all" || appState.searchQuery) {
      todoStore.miniItems = await api.listTodos("active", "");
    } else {
      todoStore.miniItems = todoStore.items.filter((t) => t.status === "active");
    }
  } catch (e) {
    toastError(e, "加载待办失败");
  }
}

export function setFilter(f: typeof todoStore.filter) {
  todoStore.filter = f;
  void loadTodos();
}

export async function selectTodo(id: string | null) {
  todoStore.selectedId = id;
  if (!id) {
    todoStore.detail = null;
    todoStore.detailResources = [];
    todoStore.detailNotes = [];
    return;
  }
  await refreshDetail(id);
}

export async function refreshDetail(id: string) {
  try {
    const [detail, resources, allNotes] = await Promise.all([
      api.getTodo(id),
      api.listResources("todo", id),
      api.listNotes("", "", "all"),
    ]);
    // 条目可能刚被删除
    if (detail.id !== todoStore.selectedId) return;
    todoStore.detail = detail;
    todoStore.detailResources = resources;
    todoStore.detailNotes = allNotes.filter((n) => n.todo_id === id);
    // 同步列表行上的标记数量
    const item = todoStore.items.find((t) => t.id === id);
    if (item) {
      item.resource_count = resources.length;
      item.note_count = todoStore.detailNotes.length;
    }
  } catch (e) {
    const shape = e as BizErrorShape;
    if (shape?.code === "not_found") {
      toast("该待办已不存在");
      await selectTodo(null);
      await loadTodos();
    } else {
      toastError(e, "加载详情失败");
    }
  }
}

export async function createTodo(input: {
  title: string;
  priority: Priority;
  next_action?: string;
  description?: string;
  due_date?: string;
}): Promise<Todo | null> {
  try {
    const todo = await api.createTodo(input);
    await loadTodos();
    appState.tab = "todo";
    await selectTodo(todo.id);
    return todo;
  } catch (e) {
    toastError(e, "新建待办失败");
    return null;
  }
}

/** 详情面板自动保存：失败时提示且保留输入内容（不回写覆盖） */
export async function patchTodo(id: string, patch: Parameters<typeof api.updateTodo>[1]) {
  try {
    const updated = await api.updateTodo(id, patch);
    if (todoStore.detail?.id === id) {
      todoStore.detail = { ...todoStore.detail, ...updated };
    }
    const item = todoStore.items.find((t) => t.id === id);
    if (item) Object.assign(item, updated);
    return updated;
  } catch (e) {
    toast(`保存失败，内容已保留：${errText(e)}`, "error");
    return null;
  }
}

export async function setTodoDone(id: string, done: boolean) {
  try {
    await api.setTodoDone(id, done);
    await loadTodos();
    if (todoStore.selectedId === id) {
      await refreshDetail(id);
    }
  } catch (e) {
    toastError(e, "更新状态失败");
  }
}

/** 拖拽/键盘排序：跨组移动会改变优先级 */
export async function moveTodo(id: string, targetPriority: Priority, beforeId: string | null) {
  try {
    const item = todoStore.items.find((t) => t.id === id);
    if (item && item.priority === targetPriority) {
      // 同组：本地乐观重排，避免闪烁
      const group = todoStore.items
        .filter((t) => t.status === "active" && t.priority === targetPriority && t.id !== id)
        .sort((a, b) => a.sort_order - b.sort_order);
      const idx = beforeId ? group.findIndex((t) => t.id === beforeId) : group.length;
      const seq = [...group.slice(0, idx < 0 ? group.length : idx), item, ...group.slice(idx < 0 ? group.length : idx)];
      seq.forEach((t, i) => (t.sort_order = (i + 1) * 1000));
    }
    await api.reorderTodo(id, targetPriority, beforeId);
    if (item && item.priority !== targetPriority) {
      item.priority = targetPriority;
    }
    await loadTodos();
    if (todoStore.selectedId === id) await refreshDetail(id);
  } catch (e) {
    toastError(e, "排序失败");
    await loadTodos();
  }
}

export async function duplicateTodo(id: string) {
  try {
    const todo = await api.duplicateTodo(id);
    await loadTodos();
    await selectTodo(todo.id);
    toast("已创建副本");
  } catch (e) {
    toastError(e, "复制条目失败");
  }
}

/** 永久删除全部已完成待办（调用前需确认） */
export async function clearCompletedTodos(): Promise<{
  deleted: number;
  detached_notes: number;
}> {
  const r = await api.clearCompletedTodos();
  if (todoStore.selectedId) {
    const sel = todoStore.items.find((t) => t.id === todoStore.selectedId);
    if (sel && sel.status === "done") await selectTodo(null);
  }
  await loadTodos();
  return { deleted: r.deleted, detached_notes: r.detached_notes };
}

export async function deleteTodo(id: string) {
  try {
    const r = await api.deleteTodo(id);
    if (todoStore.selectedId === id) await selectTodo(null);
    await loadTodos();
    toast(
      r.detached_notes > 0
        ? `已删除待办（${r.detached_notes} 条关联笔记已解除关联并保留）`
        : "已删除待办",
    );
  } catch (e) {
    toastError(e, "删除待办失败");
  }
}
