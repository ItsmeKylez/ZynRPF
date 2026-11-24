import { useEffect, useState } from "react";
import * as api from "./api";
import { Entry } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes, hex } from "./format";
import { HexViewer } from "./HexViewer";
import { m } from "./messages";
import { parseRsc7, Rsc7 } from "./rsc7";
import { formatBytes as fb } from "./format";

const MAX_DL = 64 * 1024 * 1024, STEP = 1024 * 1024;
const detectedLabel = (d?: string) => (d && (m as any)["detected_" + d]) || d || "";

export function EntryDetail({ versionId, entry, onOpenNested }: { versionId: string; entry: Entry | null; onOpenNested?: (versionId: string) => void }) {
  const [full, setFull] = useState<Entry | null>(null);
  const [err, setErr] = useState<Shown | null>(null);
  const [dl, setDl] = useState<number | null>(null);
  const [opening, setOpening] = useState(false);
  const [rsc, setRsc] = useState<Rsc7 | null>(null);
  useEffect(() => {
    setRsc(null);
    if (!entry || entry.kind !== "resource" || entry.large_resource || entry.stored_size < 16) return;
    let live = true;
    api.entryBytes(versionId, entry.id, 0, 15).then((b) => live && setRsc(parseRsc7(b, entry.sys_flags, entry.gfx_flags))).catch(() => {});
    return () => { live = false; };
  }, [versionId, entry?.id]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    setFull(null); setErr(null); setDl(null);
    if (!entry) return;
    let live = true;
    api.getEntry(versionId, entry.id).then((e) => live && setFull(e)).catch((e) => live && setErr(toShown(e)));
    return () => { live = false; };
  }, [versionId, entry?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!entry) return <section className="panel" aria-labelledby="detail-h"><h2 id="detail-h">{m.detailHeading}</h2><p className="muted">{m.noSelection}</p></section>;
  const e = full || entry;
  const download = async () => {
    setDl(0);
    try {
      const parts: Uint8Array[] = [];
      for (let o = 0; o < e.stored_size; o += STEP) {
        parts.push(await api.entryBytes(versionId, e.id, o, Math.min(e.stored_size, o + STEP) - 1));
        setDl(Math.floor((Math.min(e.stored_size, o + STEP) / e.stored_size) * 100));
      }
      const url = URL.createObjectURL(new Blob(parts as BlobPart[]));
      const a = document.createElement("a"); a.href = url; a.download = e.name || "entry.bin"; a.click();
      setTimeout(() => URL.revokeObjectURL(url), 10000);
    } catch (x) { setErr(toShown(x)); }
    setDl(null);
  };
  const rows: [string, string][] = [
    [m.fPath, e.path || e.name], [m.fKind, e.kind === "directory" ? m.kindDirectory : e.kind === "resource" ? m.kindResource : m.kindFile],
    [m.fDetected, detectedLabel(e.detected_type)], [m.fStored, formatBytes(e.stored_size) + ` (${e.stored_size} B)`],
    [m.fUncompressed, formatBytes(e.uncompressed_size)], [m.fCompressed, e.compressed ? m.yes : m.no],
    [m.fOffset, e.offset.toString() + " (0x" + hex(e.offset, 1) + ")"],
    [m.fChildren, e.child_count === null ? "" : String(e.child_count)],
    [m.fSysFlags, e.sys_flags === null ? "" : "0x" + hex(e.sys_flags, 8)], [m.fGfxFlags, e.gfx_flags === null ? "" : "0x" + hex(e.gfx_flags, 8)],
    [m.fEncryption, e.encryption_tag ? "0x" + hex(e.encryption_tag, 8) : m.none],
  ].filter((r) => r[1] !== "") as [string, string][];
  return (
    <section className="panel" aria-labelledby="detail-h">
      <h2 id="detail-h">{m.detailHeading}</h2>
      <p className="entry-name">{e.name || m.rootName}</p>
      {err && <ErrorBox err={err} />}
      <dl>{rows.map(([k, v]) => (<div key={k} className="kv"><dt>{k}</dt><dd>{v}</dd></div>))}</dl>
      {e.kind !== "directory" && !e.large_resource && e.stored_size > 0 && (
        e.stored_size <= MAX_DL
          ? <button onClick={() => void download()} disabled={dl !== null}>{dl === null ? m.downloadEntry : m.downloading(dl)}</button>
          : <p className="muted">{m.entryTooLargeToDownload(formatBytes(MAX_DL))}</p>
      )}
      {full && full.detected_type === "nested_rpf" && onOpenNested && (
        <button disabled={opening} onClick={() => { setOpening(true); api.openNested(versionId, full.id).then((r) => onOpenNested(r.version_id)).catch((x) => setErr(toShown(x))).finally(() => setOpening(false)); }}>{opening ? m.openingNested : m.openNestedButton}</button>
      )}
      {entry.kind === "resource" && !entry.large_resource && entry.stored_size >= 16 && (
        <div className="rsc" role="group" aria-label={m.rscHeading}>
          <h3>{m.rscHeading}</h3>
          {!rsc ? <p className="muted">{m.rscLoading}</p> : (
            <>
              <p><span className="badge">{rsc.magic ? m.rscMagicOk : m.rscMagicBad}</span> {rsc.magic && rsc.flagsMatchEntry !== null && <span className="badge">{rsc.flagsMatchEntry ? m.rscMatch : m.rscMismatch}</span>}</p>
              {rsc.magic && <dl className="facts">
                <dt>{m.rscVersion}</dt><dd>{rsc.version}</dd>
                <dt>{m.rscSys}</dt><dd>{fb(rsc.sysSize)}</dd>
                <dt>{m.rscGfx}</dt><dd>{fb(rsc.gfxSize)}</dd>
              </dl>}
              <p className="muted">{m.rscNote}</p>
            </>
          )}
        </div>
      )}
      <HexViewer versionId={versionId} entry={e} />
    </section>
  );
}
