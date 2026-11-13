/** 全局 UI 状态（Svelte 5 runes） */
import { api } from "../lib/api";
import { toastError } from "./toast.svelte";

export const appState = $state({
  /** normal=完整窗口 compact=简洁窗 capsule=胶囊悬浮 */
  mode: "normal" as "normal" | "compact" | "capsule",
  tab: "todo" as "todo" | "notes",
  searchQuery: "",
  settingsOpen: false,
  todoDialogOpen: false,
  compactOnTop: false,
  /** 胶囊收起状态："": 展开态 | "left" | "right" */
  capsuleDocked: "",
  /** 全局快捷键降级提示（只弹一次） */
  shortcutWarningShown: false,
});

/**
 * 切换窗口形态：乐观更新前端状态（避免后端 mode-changed 事件偶发丢失导致
 * 窗口已变大而界面仍渲染旧布局），失败时回滚；事件到达后以事件为准。
 */
export async function switchMode(mode: "normal" | "compact" | "capsule") {
  const prev = appState.mode;
  if (appState.mode === mode) return;
  appState.mode = mode;
  try {
    await api.setWindowMode(mode);
  } catch (e) {
    appState.mode = prev;
    toastError(e, "切换窗口形态失败");
  }
}
