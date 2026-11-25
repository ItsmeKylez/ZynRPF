import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { Entry, TexInfo } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";

type Fit = "stretch" | "fit" | "fill";
const NEUTRAL = { hue: 0, saturation: 1, brightness: 0, contrast: 1, tint: "#e5243b", strength: 0 };

/** One lazily loaded thumbnail (loads when it scrolls into view). */
function Thumb({ versionId, entryId, t, stamp, base }: { versionId: string; entryId: string; t: TexInfo; stamp: number; base: number | null }) {
  const ref = useRef<HTMLDivElement>(null);
  const [url, setUrl] = useState<string | null>(null);
  const [seen, setSeen] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === "undefined") { setSeen(true); return; }
    const o = new IntersectionObserver((e) => { if (e.some((x) => x.isIntersecting)) { setSeen(true); o.disconnect(); } });
    o.observe(el);
    return () => o.disconnect();
  }, []);
  useEffect(() => {
    if (!seen || !t.supported) return;
    let dead = false;
    const mip = Math.min(t.levels - 1, Math.max(0, Math.floor(Math.log2(Math.max(t.width, t.height) / 64))));
    api.texPngUrl(versionId, entryId, t.index, mip, undefined, base).then((u) => { if (dead) URL.revokeObjectURL(u); else setUrl(u); }).catch(() => {});
    return () => { dead = true; };
  }, [seen, versionId, entryId, t.index, t.supported, t.levels, t.width, t.height, stamp]);
  // Revoke an object URL only after it has been replaced, so the picture on screen never points at a revoked URL.
  useEffect(() => () => { if (url) URL.revokeObjectURL(url); }, [url]);
  return <div ref={ref} className="tex-thumb">{url ? <img src={url} alt="" /> : <span className="muted">{t.supported ? "" : m.texUnsupportedShort}</span>}</div>;
}

/** Views and edits the textures of a .ytd (or the textures inside a model or particle file): preview, export, replace with an image, recolor. Queues a Replace operation. */
export function TextureEditor({ workspaceId, versionId, entry, path, maxBlob, pendingOps, onQueued }: {
  workspaceId: string; versionId: string; entry: Entry; path: string; maxBlob: number; pendingOps: number; onQueued: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<Shown | null>(null);
  const [list, setList] = useState<{ embedded: boolean; textures: TexInfo[] } | null>(null);
  const [query, setQuery] = useState("");
  const [shown, setShown] = useState(40);
  const [sel, setSel] = useState<number | null>(null);
  const [checked, setChecked] = useState<Set<number>>(new Set());
  const [mip, setMip] = useState(0);
  const [big, setBig] = useState<string | null>(null);
  const [fit, setFit] = useState<Fit>("stretch");
  const [c, setC] = useState(NEUTRAL);
  const [note, setNote] = useState("");
  const [stamp, setStamp] = useState(0);
  // Id of the content queued by this panel: later edits and previews build on it, so a second edit does not undo the first.
  const [baseBlob, setBaseBlob] = useState<number | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const ext = entry.name.toLowerCase().split(".").pop() ?? "";
  useEffect(() => { setOpen(false); setErr(null); setList(null); setSel(null); setChecked(new Set()); setNote(""); setQuery(""); setShown(40); setBig(null); setC(NEUTRAL); setBaseBlob(null); }, [versionId, entry.id]);
  useEffect(() => { if (pendingOps === 0) setBaseBlob(null); }, [pendingOps]);
  useEffect(() => {
    if (!open || sel === null) { setBig(null); return; }
    let dead = false;
    setBig(null);
    api.texPngUrl(versionId, entry.id, sel, mip, undefined, baseBlob).then((u) => { if (dead) URL.revokeObjectURL(u); else setBig(u); }).catch((e) => { if (!dead) setErr(toShown(e)); });
    return () => { dead = true; };
  }, [open, sel, mip, versionId, entry.id, stamp, baseBlob]);

  useEffect(() => () => { if (big) URL.revokeObjectURL(big); }, [big]);

  if (entry.large_resource || !["ytd", "ydr", "ydd", "yft", "ypt"].includes(ext)) return null;

  const load = async () => {
    setBusy(true); setErr(null); setNote("");
    try { const r = await api.texList(versionId, entry.id, undefined, baseBlob); setList(r); setOpen(true); setSel(r.textures.length ? 0 : null); setMip(0); setStamp((s) => s + 1); }
    catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  };
  const queueBytes = async (bytes: Uint8Array, msg: string) => {
    if (bytes.length > maxBlob) { setErr({ message: m.contentTooLarge(formatBytes(bytes.length), formatBytes(maxBlob)), next: m.contentTooLargeNext }); return; }
    const blob = await api.putBlob(workspaceId, new Blob([bytes as BlobPart]), () => {}, new AbortController().signal);
    await api.addOp(workspaceId, versionId, { op: "replace", path, blob_id: blob.blob_id });
    setBaseBlob(blob.blob_id); setStamp((x) => x + 1); setNote(msg); onQueued();
  };
  const colourQuery = () => {
    const q = new URLSearchParams();
    if (c.hue) q.set("hue", String(c.hue));
    if (c.saturation !== 1) q.set("saturation", String(c.saturation));
    if (c.brightness) q.set("brightness", String(c.brightness));
    if (c.contrast !== 1) q.set("contrast", String(c.contrast));
    if (c.strength > 0) { q.set("tint", c.tint.slice(1)); q.set("tint_strength", String(c.strength)); }
    return q;
  };
  const colourChanged = c.hue !== 0 || c.saturation !== 1 || c.brightness !== 0 || c.contrast !== 1 || c.strength > 0;
  const run = async (f: () => Promise<void>) => { setBusy(true); setErr(null); setNote(""); try { await f(); } catch (e) { setErr(toShown(e)); } finally { setBusy(false); } };
  const recolor = (ids: number[]) => run(async () => {
    const q = colourQuery(); q.set("indices", ids.join(","));
    await queueBytes(await api.texEdit(versionId, entry.id, `recolor?${q}`, undefined, baseBlob), m.texRecolored(ids.length));
  });
  const replace = (file: File) => run(async () => {
    if (sel === null) return;
    const q = colourQuery(); q.set("fit", fit);
    await queueBytes(await api.texEdit(versionId, entry.id, `${sel}/replace?${q}`, file, baseBlob), m.texReplaced);
  });
  const download = (blob: Blob, name: string) => { const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = name; document.body.appendChild(a); a.click(); a.remove(); setTimeout(() => URL.revokeObjectURL(a.href), 1000); };
  const textures = (list?.textures ?? []).filter((t) => !query || t.name.toLowerCase().includes(query.toLowerCase()));
  const cur = list && sel !== null ? list.textures[sel] : null;

  return (
    <section className="panel" aria-labelledby="tex-h">
      <h2 id="tex-h">{m.texHeading}</h2>
      {!open && <button disabled={busy} onClick={() => void load()}>{busy ? m.texLoading : m.texOpen}</button>}
      {err && <ErrorBox err={err} />}
      {open && list && (
        <>
          <p className="muted">{m.texSummary(list.textures.length)}{list.embedded ? " " + m.texEmbeddedNote : ""}</p>
          {note && <p role="status">{note}</p>}
          <div className="ypt-toolbar">
            <input type="search" aria-label={m.texSearchLabel} placeholder={m.texSearchPlaceholder} value={query} onChange={(e) => { setQuery(e.target.value); setShown(40); }} />
            <button className="secondary" onClick={() => setChecked(new Set(textures.filter((t) => t.supported).map((t) => t.index)))}>{m.texCheckAll(textures.length)}</button>
            <button className="secondary" disabled={!checked.size} onClick={() => setChecked(new Set())}>{m.texCheckNone}</button>
          </div>
          <div className="tex-layout">
            <div className="tex-list" role="list" aria-label={m.texListLabel}>
              {!textures.length && <p className="muted">{m.texNoMatch}</p>}
              {textures.slice(0, shown).map((t) => (
                <div key={t.index} role="listitem" className={"tex-item" + (sel === t.index ? " selected" : "")}>
                  <input type="checkbox" aria-label={m.texCheckLabel(t.name)} disabled={!t.supported} checked={checked.has(t.index)} onChange={(e) => setChecked((s) => { const n = new Set(s); if (e.target.checked) n.add(t.index); else n.delete(t.index); return n; })} />
                  <button className="secondary tex-pick" aria-pressed={sel === t.index} onClick={() => { setSel(t.index); setMip(0); }}>
                    <Thumb versionId={versionId} entryId={entry.id} t={t} stamp={stamp} base={baseBlob} />
                    <span><b>{t.name}</b><br /><small>{t.width}x{t.height} {t.format}, {m.texLevels(t.levels)}</small></span>
                  </button>
                </div>
              ))}
              {textures.length > shown && <button className="secondary" onClick={() => setShown(shown + 40)}>{m.texMore(textures.length - shown)}</button>}
            </div>
            <div className="tex-detail">
              {cur && (
                <>
                  <h3>{cur.name} <small>{cur.width}x{cur.height} {cur.format}</small></h3>
                  {!cur.supported && <p className="muted">{m.texUnsupported(cur.format)}</p>}
                  {cur.supported && (
                    <>
                      <label>{m.texMip} <select value={mip} onChange={(e) => setMip(Number(e.target.value))}>{Array.from({ length: cur.levels }, (_, k) => <option key={k} value={k}>{k}: {Math.max(1, cur.width >> k)}x{Math.max(1, cur.height >> k)}</option>)}</select></label>
                      <div className="tex-big">{big ? <img src={big} alt={m.texPreviewAlt(cur.name)} /> : <span className="muted">{m.texLoading}</span>}</div>
                      <div className="row">
                        <button className="secondary" disabled={busy} onClick={() => void run(async () => { const r = await api.raw("GET", `/versions/${versionId}/entries/${entry.id}/textures/${cur.index}/png?mip=${mip}${baseBlob ? `&base_blob=${baseBlob}` : ""}`); download(await r.blob(), `${cur.name}.png`); })}>{m.texExportPng}</button>
                        <button className="secondary" disabled={busy} onClick={() => void run(async () => download(await api.texDds(versionId, entry.id, cur.index, baseBlob), `${cur.name}.dds`))}>{m.texExportDds}</button>
                      </div>
                      <div className="row">
                        <input ref={fileRef} type="file" accept="image/png,image/jpeg,image/webp,image/bmp,.tga,.dds" aria-label={m.texReplaceLabel} onChange={(e) => { const f = e.target.files?.[0]; if (f) void replace(f); if (fileRef.current) fileRef.current.value = ""; }} disabled={busy} />
                        <label>{m.texFit} <select value={fit} onChange={(e) => setFit(e.target.value as Fit)}><option value="stretch">{m.texFitStretch}</option><option value="fit">{m.texFitFit}</option><option value="fill">{m.texFitFill}</option></select></label>
                      </div>
                      <p className="muted">{m.texReplaceNote}</p>
                    </>
                  )}
                </>
              )}
              <fieldset className="tex-colour">
                <legend>{m.texColourHeading}</legend>
                <label>{m.texHue} <input type="range" min={-180} max={180} step={1} value={c.hue} onChange={(e) => setC({ ...c, hue: Number(e.target.value) })} /> <output>{c.hue}</output></label>
                <label>{m.texSaturation} <input type="range" min={0} max={3} step={0.05} value={c.saturation} onChange={(e) => setC({ ...c, saturation: Number(e.target.value) })} /> <output>{c.saturation.toFixed(2)}</output></label>
                <label>{m.texBrightness} <input type="range" min={-1} max={1} step={0.05} value={c.brightness} onChange={(e) => setC({ ...c, brightness: Number(e.target.value) })} /> <output>{c.brightness.toFixed(2)}</output></label>
                <label>{m.texContrast} <input type="range" min={0} max={3} step={0.05} value={c.contrast} onChange={(e) => setC({ ...c, contrast: Number(e.target.value) })} /> <output>{c.contrast.toFixed(2)}</output></label>
                <label>{m.texTint} <input type="color" value={c.tint} onChange={(e) => setC({ ...c, tint: e.target.value })} /></label>
                <label>{m.texTintStrength} <input type="range" min={0} max={1} step={0.05} value={c.strength} onChange={(e) => setC({ ...c, strength: Number(e.target.value) })} /> <output>{c.strength.toFixed(2)}</output></label>
                <div className="row">
                  <button disabled={busy || !colourChanged || sel === null || !cur?.supported} onClick={() => void recolor([sel as number])}>{m.texRecolorOne}</button>
                  <button disabled={busy || !colourChanged || !checked.size} onClick={() => void recolor([...checked].sort((a, b) => a - b))}>{m.texRecolorChecked(checked.size)}</button>
                  <button className="secondary" disabled={!colourChanged} onClick={() => setC(NEUTRAL)}>{m.texColourReset}</button>
                </div>
                <p className="muted">{m.texColourNote}</p>
              </fieldset>
            </div>
          </div>
          <div className="row"><button className="secondary" disabled={busy} onClick={() => setOpen(false)}>{m.texClose}</button></div>
        </>
      )}
    </section>
  );
}
