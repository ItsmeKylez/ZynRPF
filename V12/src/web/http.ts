// Typed HTTP client for /api/v1 (see docs/M1_API_CONTRACT.md and docs/M2_SLICE_NOTES.md).
// The session cookie is HttpOnly and set by the server; the page never sees the token.
import type { WireBackup, WireDocument, WireEntryView, WireFile, WireOpenDocument, WireSaveResult, WireSession, WireSettings } from "./wire";

export class ApiError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly status: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

const MUTATING = { "X-Requested-With": "zyn" } as const;

async function request(path: string, init: RequestInit = {}): Promise<Response> {
  let res: Response;
  try {
    res = await fetch("/api/v1" + path, { credentials: "same-origin", ...init });
  } catch {
    throw new ApiError("NETWORK", "Cannot reach the server. Check your connection and try again.", 0);
  }
  if (!res.ok) {
    let code = "HTTP_" + res.status;
    let message = `Request failed (${res.status}).`;
    try {
      const body = await res.json();
      if (body?.error?.code) code = String(body.error.code);
      if (body?.error?.message) message = String(body.error.message);
    } catch {
      /* body was not the JSON error shape; keep the generic message */
    }
    throw new ApiError(code, message, res.status);
  }
  return res;
}

const json = async <T>(p: string, init?: RequestInit): Promise<T> => (await request(p, init)).json() as Promise<T>;

const JSON_MUT = { ...MUTATING, "Content-Type": "application/json" } as const;

/** Optional optimistic-concurrency header: the server answers 412 STALE_DOCUMENT if the document changed since `rev`. */
const ifMatch = (rev?: number): Record<string, string> => (rev === undefined ? {} : { "If-Match": String(rev) });

const post = <T>(p: string, body: unknown, rev?: number) =>
  json<T>(p, { method: "POST", headers: { ...JSON_MUT, ...ifMatch(rev) }, body: JSON.stringify(body) });

const doc = (id: number) => `/documents/${id}`;

/** Triggers a browser download of a response body (used for POST exports, which cannot be plain links). */
export function saveBlob(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

function fileNameFrom(res: Response, fallback: string): string {
  const cd = res.headers.get("Content-Disposition") ?? "";
  const star = /filename\*=UTF-8''([^;]+)/i.exec(cd);
  if (star) {
    try {
      return decodeURIComponent(star[1]);
    } catch {
      /* fall through to the plain filename */
    }
  }
  return /filename="([^"]+)"/i.exec(cd)?.[1] ?? fallback;
}

export const http = {
  session: () => json<WireSession>("/session"),
  listFiles: () => json<WireFile[]>("/files"),
  async upload(file: File): Promise<WireFile> {
    const fd = new FormData();
    fd.append("file", file, file.name);
    const out = await json<WireFile[]>("/files", { method: "POST", headers: MUTATING, body: fd });
    return out[0];
  },
  openDocument: (fileId: string) =>
    json<WireDocument>("/documents", {
      method: "POST",
      headers: { ...MUTATING, "Content-Type": "application/json" },
      body: JSON.stringify({ fileId }),
    }),
  getDocument: (docId: number) => json<WireDocument>(`/documents/${docId}`),
  listDocuments: () => json<WireOpenDocument[]>("/documents"),
  async closeDocument(docId: number): Promise<void> {
    await request(`/documents/${docId}`, { method: "DELETE", headers: MUTATING });
  },
  async deleteSession(): Promise<void> {
    await request("/session", { method: "DELETE", headers: MUTATING });
  },
  entryView: (id: number, path: string) => json<WireEntryView>(`${doc(id)}/entries?path=${encodeURIComponent(path)}`),
  settings: (id: number, path: string) => json<WireSettings>(`${doc(id)}/settings?path=${encodeURIComponent(path)}`),
  putSettings: (id: number, path: string, values: [string, string][], rev?: number) =>
    json<WireDocument>(`${doc(id)}/settings`, { method: "PUT", headers: { ...JSON_MUT, ...ifMatch(rev) }, body: JSON.stringify({ path, values }) }),
  putText: (id: number, path: string, text: string, rev?: number) =>
    json<WireDocument>(`${doc(id)}/entries/text`, { method: "PUT", headers: { ...JSON_MUT, ...ifMatch(rev) }, body: JSON.stringify({ path, text }) }),
  createText: (id: number, path: string, text: string, rev?: number) => post<WireDocument>(`${doc(id)}/entries/text`, { path, text }, rev),
  deleteEntries: (id: number, paths: string[], rev?: number) => post<WireDocument>(`${doc(id)}/delete`, { paths }, rev),
  rename: (id: number, path: string, newName: string, rev?: number) => post<WireDocument>(`${doc(id)}/rename`, { path, newName }, rev),
  mkdir: (id: number, path: string, rev?: number) => post<WireDocument>(`${doc(id)}/mkdir`, { path }, rev),
  move: (id: number, paths: string[], dir: string, rev?: number) => post<WireDocument>(`${doc(id)}/move`, { paths, dir }, rev),
  undo: (id: number, rev?: number) => json<WireDocument>(`${doc(id)}/undo`, { method: "POST", headers: { ...MUTATING, ...ifMatch(rev) } }),
  redo: (id: number, rev?: number) => json<WireDocument>(`${doc(id)}/redo`, { method: "POST", headers: { ...MUTATING, ...ifMatch(rev) } }),
  save: (id: number, rev?: number) => json<WireSaveResult>(`${doc(id)}/save`, { method: "POST", headers: { ...MUTATING, ...ifMatch(rev) } }),
  /** Each file part's name is its relative path inside `dir`. */
  importFiles(id: number, dir: string, items: { rel: string; file: File }[], rev?: number): Promise<WireDocument> {
    const fd = new FormData();
    for (const it of items) fd.append("file", it.file, it.rel);
    return json<WireDocument>(`${doc(id)}/import?dir=${encodeURIComponent(dir)}`, { method: "POST", headers: { ...MUTATING, ...ifMatch(rev) }, body: fd });
  },
  replaceFile(id: number, path: string, file: File, rev?: number): Promise<WireDocument> {
    const fd = new FormData();
    fd.append("file", file, file.name);
    return json<WireDocument>(`${doc(id)}/replace?path=${encodeURIComponent(path)}`, { method: "POST", headers: { ...MUTATING, ...ifMatch(rev) }, body: fd });
  },
  /** Downloads a ZIP of the selected paths (`[""]` = whole archive). Resolves to the number of files in the ZIP. */
  async exportZip(id: number, paths: string[]): Promise<number> {
    const res = await request(`${doc(id)}/export`, { method: "POST", headers: JSON_MUT, body: JSON.stringify({ paths }) });
    const count = Number(res.headers.get("X-Zyn-File-Count") ?? "0");
    saveBlob(await res.blob(), fileNameFrom(res, "export.zip"));
    return Number.isFinite(count) ? count : 0;
  },
  listBackups: (fileId: string) => json<WireBackup[]>(`/files/${encodeURIComponent(fileId)}/backups`),
  restoreBackup: (id: number, backupId: string) => post<WireDocument>(`${doc(id)}/restore`, { backupId }),
  backupUrl: (fileId: string, backupId: string) => `/api/v1/files/${encodeURIComponent(fileId)}/backups/${encodeURIComponent(backupId)}/download`,
  /** Browser-navigable URLs (GET, cookie-authenticated). */
  exportUrl: (docId: number) => `/api/v1/documents/${docId}/export`,
  entryUrl: (docId: number, path: string) => `/api/v1/documents/${docId}/entry?path=${encodeURIComponent(path)}`,
  fileUrl: (fileId: string, version: "original" | "current" = "original") => `/api/v1/files/${encodeURIComponent(fileId)}/download?version=${version}`,
};
