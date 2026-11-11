/** 轻量 toast 通知 */

export interface ToastItem {
  id: number;
  text: string;
  kind: "info" | "error";
}

export const toastState = $state<{ items: ToastItem[] }>({ items: [] });

let nextId = 1;

export function toast(text: string, kind: "info" | "error" = "info") {
  const id = nextId++;
  toastState.items.push({ id, text, kind });
  setTimeout(() => {
    const idx = toastState.items.findIndex((t) => t.id === id);
    if (idx >= 0) toastState.items.splice(idx, 1);
  }, 2600);
}

export function toastError(e: unknown, prefix?: string) {
  const msg = typeof e === "object" && e !== null && "message" in e ? String((e as { message: unknown }).message) : String(e);
  toast(prefix ? `${prefix}：${msg}` : msg, "error");
}
