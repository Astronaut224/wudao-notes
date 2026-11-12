/** 全局 UI 状态（Svelte 5 runes） */
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
