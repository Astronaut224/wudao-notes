/** 与 Rust 服务层结构一一对应 */

export type Priority = 1 | 2 | 3;

export interface Todo {
  id: string;
  title: string;
  next_action: string;
  priority: Priority;
  status: "active" | "done";
  due_date: string | null;
  sort_order: number;
  description: string;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface TodoSummary extends Todo {
  resource_count: number;
  note_count: number;
}

export interface TodoPatch {
  title?: string;
  next_action?: string;
  priority?: Priority;
  due_date?: string;
  description?: string;
}

export interface Note {
  id: string;
  title: string;
  body: string;
  category: string | null;
  is_pinned: boolean;
  is_archived: boolean;
  todo_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface NoteSummary extends Note {
  resource_count: number;
  todo_title: string | null;
}

export interface NotePatch {
  title?: string;
  body?: string;
  category?: string;
  is_pinned?: boolean;
  is_archived?: boolean;
  todo_id?: string;
}

export type ResourceKind = "file" | "image" | "url";

export interface Resource {
  id: string;
  owner_type: "todo" | "note";
  todo_id: string | null;
  note_id: string | null;
  kind: ResourceKind;
  display_name: string;
  url: string | null;
  file_mode: "copy" | "reference" | null;
  stored_rel_path: string | null;
  original_path: string | null;
  file_size: number | null;
  created_at: string;
  exists: boolean;
}

/** Rust BizError 序列化结果 */
export interface BizErrorShape {
  code: string;
  message: string;
}

export function isBizError(e: unknown): e is BizErrorShape {
  return (
    typeof e === "object" &&
    e !== null &&
    "code" in e &&
    "message" in e &&
    typeof (e as BizErrorShape).message === "string"
  );
}

export function errText(e: unknown): string {
  if (isBizError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}
