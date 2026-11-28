// Backups of one workspace file (web version). Every save keeps the previous saved version here (last 20 per file).
// The uploaded original is never modified and is always available through "Download original".
import { useCallback, useEffect, useState } from "react";
import { boyutYaz, type BelgeOzet } from "../api";
import { Ikon, useArayuz } from "../ortak";
import { http } from "./http";
import { restoreBackupInto } from "./transport";
import type { WireBackup } from "./wire";

export function Backups({ belge, fileId, kapat, guncelle }: { belge: BelgeOzet; fileId: string; kapat: () => void; guncelle: (o: BelgeOzet) => void }) {
  const ui = useArayuz();
  const [list, setList] = useState<WireBackup[] | null>(null);
  const load = useCallback(() => http.listBackups(fileId).then(setList, (e) => ui.hata(e)), [fileId, ui]);
  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fileId]);

  const restore = async (b: WireBackup) => {
    const when = new Date(b.createdUnix * 1000).toLocaleString("en-GB");
    const unsaved = belge.kirli ? "\nYour unsaved changes in this document will be lost." : "";
    if (!(await ui.onayla(`Restore the backup from ${when}?`, `It becomes the current version of ${belge.ad}. The version it replaces is backed up first.${unsaved}`, "Restore", true))) return;
    const o = await ui.mesgul("Restoring", () => restoreBackupInto(belge.id, b.backupId));
    if (!o) return;
    guncelle(o);
    ui.bildir("Backup restored", "tamam");
    load();
  };

  return (
    <div className="perde" onMouseDown={(e) => e.target === e.currentTarget && kapat()}>
      <div className="modal genis-modal">
        <div className="kart-ust">
          <h3 style={{ flex: 1 }}>
            <Ikon ad="yedek" /> Backups of {belge.ad}
          </h3>
          <button className="ikon-dugme" title="Close" onClick={kapat}>
            <Ikon ad="x" />
          </button>
        </div>
        <p>
          Each save keeps the version it replaces here (the last 20 are kept). Your uploaded original is never changed.{" "}
          <a href={http.fileUrl(fileId, "original")}>Download original</a> · <a href={http.fileUrl(fileId, "current")}>Download saved version</a>
        </p>
        <div className="yedek-listesi">
          {list && !list.length && <span className="soluk">No backups yet. A backup appears when you save a file that was already saved once.</span>}
          {(list ?? []).map((b) => (
            <div key={b.backupId} className="dosya-satiri">
              <Ikon ad="arsiv" />
              <span className="ad">{new Date(b.createdUnix * 1000).toLocaleString("en-GB")}</span>
              <small>{boyutYaz(b.size)}</small>
              <a className="dugme kucuk" href={http.backupUrl(fileId, b.backupId)}>
                Download
              </a>
              <button className="dugme kucuk" onClick={() => restore(b)}>
                Restore
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
