import { useEffect, useState } from "react";
import * as api from "./api";
import { Entry } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";
import { checkXml, countMatches, looksLikeXml, replaceAll } from "./textcheck";
import { decodeText, encodeText, MAX_TEXT_BYTES, Decoded } from "./textfile";

/** Edits one text file: loads it (inflating if needed), lets the person change it, and queues a Replace operation. Saving the archive stays in the edit panel. */
export function TextEditor({ workspaceId, versionId, entry, path, maxBlob, onQueued }: {
  workspaceId: string; versionId: string; entry: Entry; path: string; maxBlob: number; onQueued: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<Shown | null>(null);
  const [meta, setMeta] = useState<Decoded | null>(null);
  const [orig, setOrig] = useState("");
  const [text, setText] = useState("");
  const [done, setDone] = useState(false);
  const [find, setFind] = useState("");
  const [repl, setRepl] = useState("");
  const [cs, setCs] = useState(true);
  const [fnote, setFnote] = useState("");
  const [xmlNote, setXmlNote] = useState("");
  useEffect(() => { setOpen(false); setErr(null); setMeta(null); setDone(false); setFind(""); setRepl(""); setFnote(""); setXmlNote(""); }, [versionId, entry.id]);

  const size = entry.uncompressed_size || entry.stored_size;
  const blocked = entry.kind !== "file" || entry.large_resource || size === 0 || (entry.compressed && entry.encryption_tag !== 0) || (!entry.compressed && !!entry.encryption_tag);
  if (blocked) return null;
  const tooLarge = size > MAX_TEXT_BYTES;
  if (tooLarge) return <p className="muted">{m.textTooLarge(formatBytes(MAX_TEXT_BYTES))}</p>;

  const load = async () => {
    setBusy(true); setErr(null); setDone(false);
    try {
      const b = await api.entryBytesDecoded(versionId, entry.id, 0, size - 1);
      const d = decodeText(b);
      if (!d) { setErr({ message: m.textNotText, next: m.textNotTextNext }); return; }
      setMeta(d); setOrig(d.text); setText(d.text); setOpen(true);
    } catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  };
  const save = async () => {
    if (!meta) return;
    setBusy(true); setErr(null);
    try {
      const bytes = encodeText(text, meta);
      if (bytes.length > maxBlob) { setErr({ message: m.contentTooLarge(formatBytes(bytes.length), formatBytes(maxBlob)), next: m.contentTooLargeNext }); return; }
      const blob = await api.putBlob(workspaceId, new Blob([bytes as BlobPart]), () => {}, new AbortController().signal);
      await api.addOp(workspaceId, versionId, { op: "replace", path, blob_id: blob.blob_id });
      setOrig(text); setDone(true); onQueued();
    } catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  };

  return (
    <section className="panel" aria-labelledby="text-h">
      <h2 id="text-h">{m.textHeading}</h2>
      {!open && <button disabled={busy} onClick={() => void load()}>{busy ? m.textLoading : m.textOpen}</button>}
      {err && <ErrorBox err={err} />}
      {open && meta && (
        <>
          <p className="muted">{m.textInfo(meta.eol === "\r\n" ? "CRLF" : "LF", meta.bom, formatBytes(encodeText(text, meta).length))}</p>
          <div className="row" role="group" aria-label={m.findHeading}>
            <label>{m.findLabel} <input type="text" value={find} onChange={(e) => { setFind(e.target.value); setFnote(""); }} /></label>
            <label>{m.replaceLabel} <input type="text" value={repl} onChange={(e) => setRepl(e.target.value)} /></label>
            <label><input type="checkbox" checked={cs} onChange={(e) => setCs(e.target.checked)} /> {m.matchCase}</label>
            <button className="secondary" disabled={!find} onClick={() => setFnote(m.findCount(countMatches(text, find, cs)))}>{m.findCountButton}</button>
            <button className="secondary" disabled={!find} onClick={() => { const r = replaceAll(text, find, repl, cs); setText(r.text); setDone(false); setFnote(m.replacedCount(r.count)); }}>{m.replaceAllButton}</button>
          </div>
          {fnote && <p role="status" className="muted">{fnote}</p>}
          <div className="row">
            <button className="secondary" onClick={() => setXmlNote(!looksLikeXml(text) ? m.xmlNotXml : checkXml(text) === null ? m.xmlOk : m.xmlBad(checkXml(text) as string))}>{m.xmlCheckButton}</button>
            {xmlNote && <span role="status" className="muted">{xmlNote}</span>}
          </div>
          <label>{m.textEditorLabel(entry.name)}
            <textarea className="texteditor" value={text} spellCheck={false} rows={18} onChange={(e) => { setText(e.target.value); setDone(false); }} />
          </label>
          <div className="row">
            <button disabled={busy || text === orig} onClick={() => void save()}>{busy ? m.textSaving : m.textQueue}</button>
            <button className="secondary" disabled={busy || text === orig} onClick={() => { setText(orig); setDone(false); }}>{m.textRevert}</button>
            <button className="secondary" disabled={busy} onClick={() => setOpen(false)}>{m.textClose}</button>
          </div>
          {done && <p role="status">{m.textQueued}</p>}
          <p className="muted">{m.textNote}</p>
        </>
      )}
    </section>
  );
}
