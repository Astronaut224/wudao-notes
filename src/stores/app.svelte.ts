/** 全局 UI 状态（Svelte 5 runes） */
export const appState = $state({
  mode: "normal" as "normal" | "compact",
  tab: "todo" as "todo" | "notes",
  searchQuery: "",
  settingsOpen: false,
  todoDialogOpen: false,
  compactOnTop: false,
  /** 全局快捷键降级提示（只弹一次） */
  shortcutWarningShown: false,
});
