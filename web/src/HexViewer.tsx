import { useEffect, useState } from "react";
import * as api from "./api";
import { Entry } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes, hexDump, parseOffset } from "./format";
import { m } from "./messages";

const PAGE = 4096;

export function HexViewer({ versionId, entry }: { versionId: string; entry: Entry }) {
  const [start, setStart] = useState(0);
  const [bytes, setBytes] = useState<Uint8Array | null>(null);
  const [err, setErr] = useState<Shown | null>(null);
  const [goto, setGoto] = useState("");
  const [bad, setBad] = useState(false);
  const size = entry.stored_size;
  const blockedReason = entry.kind === "directory" ? m.hexFolder : entry.large_resource ? m.hexLarge : entry.compressed ? m.hexCompressed : size === 0 ? m.hexEmpty : null;

  useEffect(() => { setStart(0); setGoto(""); setBad(false); }, [entry.id, versionId]);
  useEffect(() => {
    if (blockedReason) { setBytes(null); return; }
    const ctl = new AbortController();
    setErr(null);
    api.entryBytes(versionId, entry.id, start, Math.min(size, start + PAGE) - 1, ctl.signal)
      .then((b) => setBytes(b))
      .catch((e) => { if ((e as Error).name !== "AbortError") { setBytes(null); setErr(toShown(e, m.hexLoadFailed)); } });
    return () => ctl.abort();
  }, [versionId, entry.id, start, size, blockedReason]);

  if (blockedReason) return <section aria-labelledby="hex-h" className="panel"><h3 id="hex-h">{m.hexHeading}</h3><p>{blockedReason}</p></section>;
  const end = Math.min(size, start + PAGE);
  return (
    <section aria-labelledby="hex-h" className="panel">
      <h3 id="hex-h">{m.hexHeading}</h3>
      <p>{m.hexRange(String(start), String(end), formatBytes(size))}</p>
      <div className="row">
        <button disabled={start === 0} onClick={() => setStart(Math.max(0, start - PAGE))}>{m.hexPrev}</button>
        <button disabled={end >= size} onClick={() => setStart(start + PAGE)}>{m.hexNext}</button>
        <form className="row" onSubmit={(e) => { e.preventDefault(); const o = parseOffset(goto); if (o === null || o >= size) { setBad(true); return; } setBad(false); setStart(Math.floor(o / PAGE) * PAGE); }}>
          <label>{m.hexGoTo} <input value={goto} onChange={(e) => setGoto(e.target.value)} aria-invalid={bad} aria-describedby={bad ? "hex-bad" : undefined} size={12} /></label>
          <button type="submit">{m.hexGoButton}</button>
        </form>
      </div>
      {bad && <p id="hex-bad" role="alert" className="invalid">{m.hexOffsetInvalid}</p>}
      {err && <ErrorBox err={err} />}
      {bytes && <pre className="hex" tabIndex={0} aria-label={m.hexLabel}>{hexDump(bytes, start)}</pre>}
    </section>
  );
}
