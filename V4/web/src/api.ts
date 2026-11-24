import { m } from "./messages";

export class ApiError extends Error {
  status: number;
  code: string;
  next?: string;
  requestId?: string;
  details?: unknown;
  retryAfter?: number;
  constructor(status: number, code: string, message: string, next?: string, requestId?: string, details?: unknown, retryAfter?: number) {
    super(message);
    this.status = status; this.code = code; this.next = next; this.requestId = requestId; this.details = details; this.retryAfter = retryAfter;
  }
}

export interface Limits { max_upload_bytes: number; chunk_size_bytes: number; max_entries: number; max_range_bytes: number; session_quota_bytes: number; idle_hours: number; max_workspaces: number; max_blob_bytes: number; max_ops: number; max_versions: number }
export interface Me { csrf_token?: string; usage_bytes: number; quota_bytes: number; workspace_count: number }
export interface Workspace { id: string; name: string; version_ids: string[]; usage_bytes: number }
export interface VersionInfo { id: string; filename: string; size: number; state: "parsing" | "ready" | "failed"; entry_count: number; origin?: "upload" | "save" | "nested"; created_at?: number }
export interface Entry {
  id: string; name: string; kind: "directory" | "file" | "resource"; stored_size: number; uncompressed_size: number; compressed: boolean;
  encryption_tag: number | null; offset: number; child_count: number | null; sys_flags: number | null; gfx_flags: number | null; large_resource: boolean;
  path?: string; detected_type?: string;
}
export interface Finding { code: string; severity: "error" | "warning" | "info"; message: string; next_step: string; detail: string }
export interface Report { ok: boolean; errors: number; warnings: number; findings: Finding[]; planned?: { entries: number; file_bytes: number; copied_entries: number; new_entries: number }; saved?: { entries: number; file_bytes: number; copied_entries: number; new_entries: number } }
export interface Job { id: string; type?: string; state: "queued" | "running" | "succeeded" | "failed"; progress: number; error: { code: string; message: string; next_step: string } | null; version_id: string | null; report?: Report }
export interface EditOp { seq: number; type: "rename" | "delete" | "mkdir" | "add_file" | "replace"; path?: string; new_name?: string; parent?: string; name?: string; size?: number; active: boolean }
export interface EditState { workspace_id: string; base_version: string | null; cursor: number; can_undo: boolean; can_redo: boolean; ops: EditOp[]; writer: "you" | "other" | "none"; saving: boolean; max_ops: number; max_versions: number; pending_content_bytes: number }

let csrf = "";
/** Identifies this browser tab for the one-writer rule (D-020). A new random id per page load; never stored. */
export const tabId = (() => { const b = new Uint8Array(12); crypto.getRandomValues(b); return Array.from(b, (x) => x.toString(16).padStart(2, "0")).join(""); })();

function toApiError(status: number, body: any, retryAfter: number | undefined): ApiError {
  const e = body && body.error;
  if (e) return new ApiError(status, e.code, e.message, e.next_step, e.request_id, e.details, retryAfter);
  return new ApiError(status, "http_" + status, m.networkError, m.networkErrorNext);
}
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function raw(method: string, path: string, opts: { body?: unknown; headers?: Record<string, string>; signal?: AbortSignal; onWait?: (s: number) => void } = {}): Promise<Response> {
  for (let attempt = 0; ; attempt++) {
    const headers: Record<string, string> = { ...(opts.headers || {}) };
    let body: BodyInit | undefined;
    if (opts.body !== undefined) { headers["Content-Type"] = "application/json"; body = JSON.stringify(opts.body); }
    if (method !== "GET") headers["X-CSRF-Token"] = csrf;
    headers["X-Tab-Id"] = tabId;
    let res: Response;
    try {
      res = await fetch("/api/v1" + path, { method, headers, body, credentials: "same-origin", signal: opts.signal });
    } catch (e) {
      if ((e as Error).name === "AbortError") throw e;
      throw new ApiError(0, "network_error", m.networkError, m.networkErrorNext);
    }
    if (res.ok) return res;
    let j: any = null;
    try { j = await res.json(); } catch { /* not JSON */ }
    const ra = Number(res.headers.get("Retry-After")) || undefined;
    if (res.status === 429 && attempt < 2 && ra && ra <= 20) {
      opts.onWait?.(ra);
      await sleep(ra * 1000);
      continue;
    }
    throw toApiError(res.status, j, ra);
  }
}
async function json<T>(method: string, path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
  const r = await raw(method, path, { body, signal });
  return (await r.json()) as T;
}

export async function startSession(): Promise<void> {
  const r = await fetch("/api/v1/sessions/anonymous", { method: "POST", credentials: "same-origin", headers: { "Content-Type": "application/json" }, body: "{}" }).catch(() => {
    throw new ApiError(0, "network_error", m.networkError, m.networkErrorNext);
  });
  const j: any = await r.json().catch(() => null);
  if (!r.ok) throw toApiError(r.status, j, Number(r.headers.get("Retry-After")) || undefined);
  csrf = j.csrf_token;
}
/** Reuses the existing session after a page reload; returns false when there is none (or it expired). */
export async function resumeSession(): Promise<boolean> {
  try {
    const r = await fetch("/api/v1/me", { credentials: "same-origin" });
    if (!r.ok) return false;
    const j: any = await r.json();
    if (typeof j.csrf_token !== "string" || !j.csrf_token) return false;
    csrf = j.csrf_token;
    return true;
  } catch { return false; }
}
export const getLimits = () => json<Limits>("GET", "/limits");
export const getMe = () => json<Me>("GET", "/me");
export const deleteMe = async () => { await raw("DELETE", "/me"); };
export const listWorkspaces = async () => (await json<{ workspaces: Workspace[] }>("GET", "/workspaces")).workspaces;
export const createWorkspace = (name: string) => json<Workspace>("POST", "/workspaces", { name });
export const deleteWorkspace = async (id: string) => { await raw("DELETE", `/workspaces/${id}`); };
export const listVersions = async (ws: string) => (await json<{ versions: VersionInfo[] }>("GET", `/workspaces/${ws}/versions`)).versions;
export const getJob = (id: string) => json<Job>("GET", `/jobs/${id}`);
export const listEntries = (v: string, p: { parent: string; cursor: number; limit: number; sort: string; order: string }) =>
  json<{ total: number; entries: Entry[]; next_cursor: string | null }>("GET", `/versions/${v}/entries?parent=${p.parent}&cursor=${p.cursor}&limit=${p.limit}&sort=${p.sort}&order=${p.order}`);
export const getEntry = (v: string, id: string) => json<Entry>("GET", `/versions/${v}/entries/${id}`);
export const openNested = (v: string, id: string) => json<{ version_id: string; entry_count: number }>("POST", `/versions/${v}/entries/${id}/open`, "{}");
export const getPath = (v: string, id: string) => json<{ path: string; chain: { id: string; name: string }[] }>("GET", `/versions/${v}/entries/${id}/path`);
export const search = (v: string, q: string) => json<{ hits: Entry[]; truncated: boolean }>("GET", `/versions/${v}/search?q=${encodeURIComponent(q)}&limit=200`);
export const downloadUrl = (v: string) => `/api/v1/versions/${v}/download`;
export async function entryBytes(v: string, id: string, start: number, end: number, signal?: AbortSignal): Promise<Uint8Array> {
  const r = await raw("GET", `/versions/${v}/entries/${id}/bytes?basis=stored`, { headers: { Range: `bytes=${start}-${end}` }, signal });
  return new Uint8Array(await r.arrayBuffer());
}

// ---------- uploads ----------
export const uploadStart = (ws: string, filename: string, size: number, sha256?: string) =>
  json<{ upload_id: string; chunk_size: number; chunk_count: number }>("POST", `/workspaces/${ws}/uploads`, sha256 ? { filename, size, sha256 } : { filename, size });
export const uploadStatus = (id: string) => json<{ size: number; chunk_size: number; received: number[]; completed: boolean }>("GET", `/uploads/${id}`);
export const uploadComplete = (id: string) => json<{ job_id: string; version_id: string }>("POST", `/uploads/${id}/complete`, {});
export const uploadAbort = async (id: string) => { await raw("DELETE", `/uploads/${id}`); };

export function putChunk(id: string, idx: number, blob: Blob, onProgress: (loaded: number) => void, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const x = new XMLHttpRequest();
    x.open("PUT", `/api/v1/uploads/${id}/chunks/${idx}`);
    x.setRequestHeader("X-CSRF-Token", csrf);
    x.upload.onprogress = (e) => onProgress(e.loaded);
    x.onload = () => {
      if (x.status >= 200 && x.status < 300) return resolve();
      let j: any = null;
      try { j = JSON.parse(x.responseText); } catch { /* ignore */ }
      reject(toApiError(x.status, j, Number(x.getResponseHeader("Retry-After")) || undefined));
    };
    x.onerror = () => reject(new ApiError(0, "network_error", m.networkError, m.networkErrorNext));
    x.onabort = () => reject(new DOMException("aborted", "AbortError"));
    signal.addEventListener("abort", () => x.abort());
    x.send(blob);
  });
}

// ---------- editing (M3) ----------
export const getEdit = (ws: string) => json<EditState>("GET", `/workspaces/${ws}/edit`);
export const claimWriter = (ws: string) => json<EditState>("POST", `/workspaces/${ws}/writer`, {});
/** Best-effort release when the page closes. */
export function releaseWriter(ws: string): void {
  try { void fetch(`/api/v1/workspaces/${ws}/writer`, { method: "DELETE", keepalive: true, credentials: "same-origin", headers: { "X-CSRF-Token": csrf, "X-Tab-Id": tabId } }); } catch { /* ignore */ }
}
export const editTree = (ws: string, version: string, path: string, cursor: number, limit: number) =>
  json<{ path: string; total: number; entries: { name: string; kind: "directory" | "file"; size: number; new: boolean }[] }>("GET",
    `/workspaces/${ws}/edit/tree?version=${encodeURIComponent(version)}&path=${encodeURIComponent(path)}&cursor=${cursor}&limit=${limit}`);
export const addOp = (ws: string, version: string, op: Record<string, unknown>) => json<EditState>("POST", `/workspaces/${ws}/edit/ops`, { version, ...op });
export const undoOp = (ws: string) => json<EditState>("POST", `/workspaces/${ws}/edit/undo`, {});
export const redoOp = (ws: string) => json<EditState>("POST", `/workspaces/${ws}/edit/redo`, {});
export const discardEdits = (ws: string) => json<EditState>("POST", `/workspaces/${ws}/edit/discard`, {});
export const checkEdits = (ws: string) => json<Report>("POST", `/workspaces/${ws}/edit/validate`, {});
export const saveEdits = (ws: string) => json<{ job_id: string }>("POST", `/workspaces/${ws}/edit/save`, {});
export const checkVersion = (v: string) => json<Report>("POST", `/versions/${v}/validate`, {});
export const deleteVersion = async (v: string) => { await raw("DELETE", `/versions/${v}`); };

export function putBlob(ws: string, file: Blob, onProgress: (loaded: number) => void, signal: AbortSignal): Promise<{ blob_id: number; size: number }> {
  return new Promise((resolve, reject) => {
    const x = new XMLHttpRequest();
    x.open("PUT", `/api/v1/workspaces/${ws}/blobs`);
    x.setRequestHeader("X-CSRF-Token", csrf);
    x.setRequestHeader("X-Tab-Id", tabId);
    x.upload.onprogress = (e) => onProgress(e.loaded);
    x.onload = () => {
      let j: any = null;
      try { j = JSON.parse(x.responseText); } catch { /* ignore */ }
      if (x.status >= 200 && x.status < 300 && j) return resolve(j);
      reject(toApiError(x.status, j, Number(x.getResponseHeader("Retry-After")) || undefined));
    };
    x.onerror = () => reject(new ApiError(0, "network_error", m.networkError, m.networkErrorNext));
    x.onabort = () => reject(new DOMException("aborted", "AbortError"));
    signal.addEventListener("abort", () => x.abort());
    x.send(file);
  });
}
