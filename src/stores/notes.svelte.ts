/** 笔记状态与操作 */
import { api } from "../lib/api";
import type { Note, NoteSummary, Resource } from "../lib/types";
import { appState } from "./app.svelte";
import { toast, toastError } from "./toast.svelte";

export const noteStore = $state({
  items: [] as NoteSummary[],
  categories: [] as string[],
  categoryFilter: "" as string,
  /** 特殊筛选项：已归档 */
  showArchived: false,
  selectedId: null as string | null,
  detail: null as Note | null,
  detailResources: [] as Resource[],
});

export async function loadNotes() {
  try {
    noteStore.items = await api.listNotes(
      appState.searchQuery,
      noteStore.categoryFilter,
      noteStore.showArchived ? "archived" : "unarchived",
    );
    noteStore.categories = await api.listNoteCategories();
  } catch (e) {
    toastError(e, "加载笔记失败");
  }
}

export function setCategoryFilter(cat: string) {
  noteStore.categoryFilter = cat;
  noteStore.showArchived = false;
  void loadNotes();
}

export function setShowArchived() {
  noteStore.showArchived = true;
  noteStore.categoryFilter = "";
  void loadNotes();
}

export async function selectNote(id: string | null) {
  noteStore.selectedId = id;
  if (!id) {
    noteStore.detail = null;
    noteStore.detailResources = [];
    return;
  }
  await refreshNote(id);
}

export async function refreshNote(id: string) {
  try {
    const [detail, resources] = await Promise.all([
      api.getNote(id),
      api.listResources("note", id),
    ]);
    if (noteStore.selectedId !== id) return;
    noteStore.detail = detail;
    noteStore.detailResources = resources;
    const item = noteStore.items.find((n) => n.id === id);
    if (item) item.resource_count = resources.length;
  } catch (e) {
    const shape = e as { code?: string };
    if (shape?.code === "not_found") {
      toast("该笔记已不存在");
      await selectNote(null);
      await loadNotes();
    } else {
      toastError(e, "加载笔记失败");
    }
  }
}

export async function createNote(input: {
  title?: string;
  body?: string;
  category?: string;
  todo_id?: string;
}): Promise<Note | null> {
  try {
    const note = await api.createNote(input);
    appState.tab = "notes";
    await loadNotes();
    await selectNote(note.id);
    return note;
  } catch (e) {
    toastError(e, "新建笔记失败");
    return null;
  }
}

export async function patchNote(id: string, patch: Parameters<typeof api.updateNote>[1]) {
  try {
    const updated = await api.updateNote(id, patch);
    if (noteStore.detail?.id === id) {
      noteStore.detail = { ...noteStore.detail, ...updated };
    }
    const item = noteStore.items.find((n) => n.id === id);
    if (item) Object.assign(item, updated);
    return updated;
  } catch (e) {
    toast(`保存失败，内容已保留：${typeof e === "object" && e && "message" in e ? String((e as { message: unknown }).message) : String(e)}`, "error");
    return null;
  }
}

/** 永久删除全部已归档笔记（调用前需确认） */
export async function clearArchivedNotes(): Promise<number> {
  const r = await api.clearArchivedNotes();
  await selectNote(null);
  await loadNotes();
  return r.deleted;
}

export async function deleteNote(id: string) {
  try {
    await api.deleteNote(id);
    if (noteStore.selectedId === id) await selectNote(null);
    await loadNotes();
    toast("已删除笔记");
  } catch (e) {
    toastError(e, "删除笔记失败");
  }
}
