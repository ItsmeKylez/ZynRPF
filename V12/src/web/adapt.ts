// Pure mappers: backend wire types -> the types the existing (Turkish-named) components already consume.
// Erasable TypeScript only (no enums/namespaces) so the tests in web-tests/ run under plain Node.
import type { BelgeOzet, DosyaIcerigi, DugumBilgi, Girdi, KayitSonucu, Paket, RenkUclusu } from "../api";
import type { WireDocument, WireEntryView, WireNode, WirePackage, WireSaveResult, WireSettings } from "./wire";

/** Opaque handle the web UI uses wherever the desktop UI passes a filesystem path. */
export const WEB_FILE_PREFIX = "web-file:";

export const webFileHandle = (fileId: string) => WEB_FILE_PREFIX + fileId;

/** Returns the fileId inside a `web-file:` handle, or null if `yol` is anything else (e.g. a real local path). */
export function fileIdFromHandle(yol: string): string | null {
  return yol.startsWith(WEB_FILE_PREFIX) && yol.length > WEB_FILE_PREFIX.length ? yol.slice(WEB_FILE_PREFIX.length) : null;
}

const KIND: Record<WireNode["kind"], DugumBilgi["tur"]> = { directory: "dizin", archive: "arsiv", file: "dosya" };

export function toNode(n: WireNode): DugumBilgi {
  const tur = KIND[n.kind];
  if (!tur) throw new Error(`Unknown node kind from server: ${String(n.kind)}`);
  return {
    ad: n.name,
    yol: n.path,
    tur,
    boyut: n.size,
    kayitli: n.storedSize,
    degisti: n.modified,
    rsc: n.resource,
    cocuklar: n.children.map(toNode),
  };
}

export function toPackage(p: WirePackage): Paket {
  return {
    ad: p.name,
    surum: p.version,
    yazar: p.author,
    aciklama: p.description,
    hedefler: p.targets.map((t) => ({ kaynak: t.source, arsivler: t.archives, yol: t.path, sil: t.delete })),
  };
}

/** `fileId` is known to the transport (it opened the document from that upload); the summary itself does not carry it. */
export function toBelgeOzet(d: WireDocument, fileId?: string): BelgeOzet {
  return {
    id: d.docId,
    yol: fileId ? webFileHandle(fileId) : null,
    ad: d.name,
    sifreleme: d.encryption,
    disk_boyutu: d.diskSize,
    tahmini_boyut: d.estimatedSize,
    kirli: d.dirty,
    geri: d.undoDepth,
    ileri: d.redoDepth,
    durum_no: d.revision,
    paket: d.package ? toPackage(d.package) : null,
    paket_hatasi: d.packageError,
    eksik_kaynak: d.missingSources,
    agac: d.tree.map(toNode),
  };
}

const ENTRY_KIND: Record<WireEntryView["kind"], DosyaIcerigi["tur"]> = {
  meta: "meta",
  texture: "ytd",
  model: "model",
  resource: "kaynak",
  setting: "ayar",
  text: "metin",
  binary: "ikili",
};

export function toDosyaIcerigi(v: WireEntryView): DosyaIcerigi {
  const tur = ENTRY_KIND[v.kind];
  if (!tur) throw new Error(`Unknown entry kind from server: ${String(v.kind)}`);
  return { tur, boyut: v.size, metin: v.text, hex: v.hexDump, rsc_surum: v.rscVersion };
}

export function toKayitSonucu(r: WireSaveResult, fileId?: string): KayitSonucu {
  return { ozet: toBelgeOzet(r.summary, fileId), eski_boyut: r.oldSize, yeni_boyut: r.newSize, yedek: r.backupId };
}

export function toSettings(s: WireSettings): { girdiler: Girdi[]; renkler: RenkUclusu[] } {
  return {
    girdiler: s.entries.map((e) => ({ anahtar: e.key, deger: e.value, satir: e.line, tekrar: e.repeat })),
    renkler: s.colors.map((c) => ({ taban: c.base, anahtarlar: c.keys, olcek: c.scale })),
  };
}

/** Handles the dialog shim returns for browser-picked files (resolved by the transport at command time). */
export const WEB_PICK_PREFIX = "web-pick:";
/** Returned instead of a folder path for "export to a folder": the web version downloads a ZIP. */
export const WEB_DOWNLOAD_HANDLE = "web-download:";
/** The only desktop dialog title that means "export destination" (src/Belge.tsx). Pinned by a test; M7 must update both. */
export const EXPORT_DIALOG_TITLE = "Nereye çıkarılsın?";
