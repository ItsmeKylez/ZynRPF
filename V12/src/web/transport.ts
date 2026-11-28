// Web replacement for Tauri `invoke`. The existing `api` object (src/api.ts) keeps calling invoke("cmd", args);
// in the web build that call lands here and is mapped onto /api/v1. Commands without a backend route yet
// (M4+) reject with UnavailableError — they are never silently faked.
import { WEB_DOWNLOAD_HANDLE, fileIdFromHandle, toBelgeOzet, toDosyaIcerigi, toKayitSonucu, toSettings } from "./adapt";
import { http } from "./http";
import { KeyedQueue } from "./queue";
import { releasePicks, resolvePicks } from "./shims/picks";
import type { WireDocument } from "./wire";
import { UnavailableError } from "./unavailable";

/** docId -> fileId, so summaries can carry the same `web-file:` handle the document was opened from. */
const fileOfDoc = new Map<number, string>();
/** docId -> last revision this page has seen; sent as If-Match so edits against a changed document are refused (412). */
const revOfDoc = new Map<number, number>();

function seen(d: WireDocument): WireDocument {
  revOfDoc.set(d.docId, d.revision);
  return d;
}
const summary = (d: WireDocument, id: number) => toBelgeOzet(seen(d), fileOfDoc.get(id));

type Args = Record<string, unknown> | undefined;

function num(a: Args, key: string): number {
  const v = a?.[key];
  if (typeof v !== "number" || !Number.isInteger(v)) throw new Error(`Internal error: "${key}" must be an integer.`);
  return v;
}

function str(a: Args, key: string): string {
  const v = a?.[key];
  if (typeof v !== "string") throw new Error(`Internal error: "${key}" must be a string.`);
  return v;
}

function strs(a: Args, key: string): string[] {
  const v = a?.[key];
  if (!Array.isArray(v) || v.some((x) => typeof x !== "string")) throw new Error(`Internal error: "${key}" must be a list of strings.`);
  return v as string[];
}

export const STALE_EVENT = "zyn:stale-document";

/** Maps a desktop command onto the backend. A refused stale edit (412) also tells the shell to reload that document. */
const queue = new KeyedQueue();

/** Commands that can remove or move entries. */
const STRUCTURAL = new Set(["sil", "yeniden_adlandir", "tasi", "geri_al", "yinele", "ice_aktar", "dosya_degistir", "metin_yaz", "metin_olustur", "dizin_olustur"]);
/** docId -> when a structural edit last succeeded. */
const lastEdit = new Map<number, number>();
/** How long after an edit a failed entry read counts as "the view was asking about the old tree". */
export const SUPERSEDED_READ_WINDOW_MS = 2000;

/**
 * The reused file viewer re-reads its entry whenever the UI re-renders (e.g. when a rename dialog closes). That read can
 * land just after the user's own rename/move/delete removed that path, because the viewer has not yet received the new tree.
 * Such a read is superseded, not an error: answer "no content" instead of toasting a spurious "not found". Reads that fail
 * with no recent edit still surface normally.
 */
export function isSupersededRead(cmd: string, code: string | undefined, sinceEditMs: number | undefined): boolean {
  return cmd === "dosya_oku" && code === "NOT_FOUND" && sinceEditMs !== undefined && sinceEditMs < SUPERSEDED_READ_WINDOW_MS;
}

export async function webInvoke(cmd: string, args?: Args): Promise<unknown> {
  const id = args?.id;
  try {
    // Commands on one document run in issue order (see queue.ts); commands without a document id run immediately.
    const out = typeof id === "number" ? await queue.run(`doc:${id}`, () => dispatch(cmd, args)) : await dispatch(cmd, args);
    if (typeof id === "number" && STRUCTURAL.has(cmd)) lastEdit.set(id, Date.now());
    return out;
  } catch (e) {
    if (typeof id === "number" && isSupersededRead(cmd, (e as { code?: string }).code, lastEdit.has(id) ? Date.now() - lastEdit.get(id)! : undefined)) return null;
    if ((e as { code?: string }).code === "STALE_DOCUMENT" && typeof id === "number") {
      revOfDoc.delete(id);
      window.dispatchEvent(new CustomEvent(STALE_EVENT, { detail: id }));
    }
    throw e;
  }
}

async function dispatch(cmd: string, args?: Args): Promise<unknown> {
  switch (cmd) {
    case "arsiv_ac": {
      const yol = str(args, "yol");
      const fileId = fileIdFromHandle(yol);
      if (!fileId) throw new UnavailableError("Opening a file by local path");
      const doc = seen(await http.openDocument(fileId));
      fileOfDoc.set(doc.docId, fileId);
      return toBelgeOzet(doc, fileId);
    }
    case "belge_ozet": {
      const id = num(args, "id");
      return summary(await http.getDocument(id), id);
    }
    case "belge_kapat": {
      const id = num(args, "id");
      await http.closeDocument(id);
      fileOfDoc.delete(id);
      revOfDoc.delete(id);
      return null;
    }
    case "acik_belgeler": {
      const open = await http.listDocuments();
      for (const d of open) fileOfDoc.set(d.docId, d.fileId);
      const out = await Promise.all(
        open.map(async (d) => ({ id: d.docId, ad: d.name, paket: (await http.getDocument(d.docId)).package?.name ?? null })),
      );
      return out;
    }
    case "ayarlar_oku":
      return {}; // no stored settings exist in the web version yet (true, not faked)

    // ---- M4: archive workflows ----
    case "dosya_oku": {
      return toDosyaIcerigi(await http.entryView(num(args, "id"), str(args, "yol")));
    }
    case "vs_oku": {
      return toSettings(await http.settings(num(args, "id"), str(args, "yol")));
    }
    case "vs_ayarla": {
      const id = num(args, "id");
      const values = args?.degerler;
      if (!Array.isArray(values) || values.some((p) => !Array.isArray(p) || p.length !== 2 || typeof p[0] !== "string" || typeof p[1] !== "string")) {
        throw new Error('Internal error: "degerler" must be a list of [key, value] pairs.');
      }
      return summary(await http.putSettings(id, str(args, "yol"), values as [string, string][], revOfDoc.get(id)), id);
    }
    case "metin_yaz": {
      const id = num(args, "id");
      return summary(await http.putText(id, str(args, "yol"), str(args, "metin"), revOfDoc.get(id)), id);
    }
    case "metin_olustur": {
      const id = num(args, "id");
      return summary(await http.createText(id, str(args, "yol"), str(args, "metin"), revOfDoc.get(id)), id);
    }
    case "sil": {
      const id = num(args, "id");
      return summary(await http.deleteEntries(id, strs(args, "yollar"), revOfDoc.get(id)), id);
    }
    case "yeniden_adlandir": {
      const id = num(args, "id");
      return summary(await http.rename(id, str(args, "yol"), str(args, "ad"), revOfDoc.get(id)), id);
    }
    case "dizin_olustur": {
      const id = num(args, "id");
      return summary(await http.mkdir(id, str(args, "yol"), revOfDoc.get(id)), id);
    }
    case "tasi": {
      const id = num(args, "id");
      return summary(await http.move(id, strs(args, "yollar"), str(args, "dizin"), revOfDoc.get(id)), id);
    }
    case "geri_al": {
      const id = num(args, "id");
      return summary(await http.undo(id, revOfDoc.get(id)), id);
    }
    case "yinele": {
      const id = num(args, "id");
      return summary(await http.redo(id, revOfDoc.get(id)), id);
    }
    case "kaydet": {
      const id = num(args, "id");
      if (args?.hedef != null) throw new UnavailableError("Saving to a chosen location");
      const r = await http.save(id, revOfDoc.get(id));
      seen(r.summary);
      return toKayitSonucu(r, fileOfDoc.get(id));
    }
    case "ice_aktar": {
      const id = num(args, "id");
      const handles = strs(args, "kaynaklar");
      const doc = await http.importFiles(id, str(args, "dizin"), resolvePicks(handles), revOfDoc.get(id));
      releasePicks(handles);
      return summary(doc, id);
    }
    case "dosya_degistir": {
      const id = num(args, "id");
      const handle = str(args, "kaynak");
      const [pick] = resolvePicks([handle]);
      const doc = await http.replaceFile(id, str(args, "yol"), pick.file, revOfDoc.get(id));
      releasePicks([handle]);
      return summary(doc, id);
    }
    case "disa_aktar": {
      if (str(args, "hedef") !== WEB_DOWNLOAD_HANDLE) throw new UnavailableError("Exporting to a local folder");
      return http.exportZip(num(args, "id"), strs(args, "yollar"));
    }
    default:
      throw new UnavailableError(`"${cmd}"`);
  }
}

/** Handle for documents restored after a page reload (the server keeps them for the session). */
export function rememberDocumentFile(docId: number, fileId: string): void {
  fileOfDoc.set(docId, fileId);
}

/** Restores a backup into an open document and returns the refreshed summary (keeps the revision bookkeeping in sync). */
export async function restoreBackupInto(docId: number, backupId: string) {
  lastEdit.set(docId, Date.now());
  return summary(await http.restoreBackup(docId, backupId), docId);
}

/** Re-reads a document from the server, e.g. after a refused stale edit (412). */
export async function refreshDocument(docId: number) {
  return summary(await http.getDocument(docId), docId);
}
