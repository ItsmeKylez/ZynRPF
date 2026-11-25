import { useEffect, useMemo, useState } from "react";
import * as api from "./api";
import { Entry, VsColour, VsEntry } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";
import { changeLines, formatNum, fromHex, matchesPreset, numRange, parseNum, PRESETS, tintColours, toHex } from "./vsbulk";

/** Edits visualsettings.dat: loads the settings from the server, lets the person change values and colors, and queues a Replace operation. */
export function VsEditor({ workspaceId, versionId, entry, path, maxBlob, pendingOps, onQueued }: {
  workspaceId: string; versionId: string; entry: Entry; path: string; maxBlob: number; pendingOps: number; onQueued: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<Shown | null>(null);
  const [data, setData] = useState<{ entries: VsEntry[]; colours: VsColour[] } | null>(null);
  const [query, setQuery] = useState("");
  const [preset, setPreset] = useState("");
  const [changedOnly, setChangedOnly] = useState(false);
  const [openGroups, setOpenGroups] = useState<Set<string>>(new Set());
  const [pending, setPending] = useState<Map<string, string>>(new Map());
  const [note, setNote] = useState("");
  // Id of the content queued by this panel: later reads and edits build on it, so a second edit does not undo the first.
  const [baseBlob, setBaseBlob] = useState<number | null>(null);
  const [tint, setTint] = useState({ color: "#e5243b", strength: 1 });
  useEffect(() => { setOpen(false); setErr(null); setData(null); setPending(new Map()); setNote(""); setQuery(""); setPreset(""); setChangedOnly(false); setOpenGroups(new Set()); setBaseBlob(null); }, [versionId, entry.id]);
  useEffect(() => { if (pendingOps === 0) setBaseBlob(null); }, [pendingOps]);

  const base = useMemo(() => new Map((data?.entries ?? []).map((e) => [e.key, e.value])), [data]);
  const valueOf = (k: string) => pending.get(k) ?? base.get(k) ?? "0";
  const setVals = (kv: Record<string, string>) => setPending((p) => {
    const n = new Map(p);
    for (const [k, v] of Object.entries(kv)) { if (v === base.get(k)) n.delete(k); else n.set(k, v); }
    return n;
  });

  const colourKeys = useMemo(() => { const s = new Map<string, VsColour>(); for (const c of data?.colours ?? []) for (const k of c.keys) s.set(k, c); return s; }, [data]);
  const activePreset = PRESETS.find((p) => p.id === preset);
  const visible = (key: string) => {
    if (query && !key.toLowerCase().includes(query.toLowerCase())) return false;
    if (activePreset && !matchesPreset(activePreset, key)) return false;
    if (changedOnly && !pending.has(key) && !(colourKeys.get(key)?.keys.some((k) => pending.has(k)))) return false;
    return true;
  };
  const groups = useMemo(() => {
    const g = new Map<string, { colours: VsColour[]; entries: VsEntry[] }>();
    for (const e of data?.entries ?? []) {
      if (!visible(e.key)) continue;
      const name = e.key.split(".")[0];
      if (!g.has(name)) g.set(name, { colours: [], entries: [] });
      const grp = g.get(name)!, c = colourKeys.get(e.key);
      if (!c) grp.entries.push(e); else if (!grp.colours.includes(c)) grp.colours.push(c);
    }
    return [...g.entries()];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, query, preset, changedOnly, pending, colourKeys]);
  const visibleColours = useMemo(() => groups.flatMap(([, g]) => g.colours), [groups]);

  const load = async () => {
    setBusy(true); setErr(null); setNote("");
    try { const r = await api.vsRead(versionId, entry.id, undefined, baseBlob); setData(r); setPending(new Map()); setOpen(true); }
    catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  };
  const queue = async () => {
    setBusy(true); setErr(null);
    try {
      const bytes = await api.vsEdit(versionId, entry.id, changeLines(pending), baseBlob);
      if (bytes.length > maxBlob) { setErr({ message: m.contentTooLarge(formatBytes(bytes.length), formatBytes(maxBlob)), next: m.contentTooLargeNext }); return; }
      const blob = await api.putBlob(workspaceId, new Blob([bytes as BlobPart]), () => {}, new AbortController().signal);
      await api.addOp(workspaceId, versionId, { op: "replace", path, blob_id: blob.blob_id });
      setBaseBlob(blob.blob_id); setData(await api.vsRead(versionId, entry.id, undefined, blob.blob_id));
      setPending(new Map()); setNote(m.vsQueued); onQueued();
    } catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  };
  const applyTint = () => {
    const next = new Map<string, string>();
    const n = tintColours(visibleColours, valueOf, fromHex(tint.color), tint.strength, next);
    setVals(Object.fromEntries(next));
    setNote(n ? m.vsTinted(n) : m.vsTintNothing);
  };

  return (
    <section className="panel" aria-labelledby="vs-h">
      <h2 id="vs-h">{m.vsHeading}</h2>
      {!open && <button disabled={busy} onClick={() => void load()}>{busy ? m.vsLoading : m.vsOpen}</button>}
      {err && <ErrorBox err={err} />}
      {open && data && (
        <>
          <p className="muted">{m.vsSummary(data.entries.length, data.colours.length)}</p>
          {note && <p role="status">{note}</p>}
          <div className="ypt-toolbar">
            <input type="search" aria-label={m.vsSearchLabel} placeholder={m.vsSearchPlaceholder} value={query} onChange={(e) => setQuery(e.target.value)} />
            <button className={changedOnly ? "" : "secondary"} aria-pressed={changedOnly} disabled={!pending.size && !changedOnly} onClick={() => setChangedOnly(!changedOnly)}>{m.vsChangedOnly(pending.size)}</button>
          </div>
          <div className="ypt-toolbar" role="group" aria-label={m.vsPresets}>
            <button className={preset === "" ? "" : "secondary"} aria-pressed={preset === ""} onClick={() => setPreset("")}>{m.vsAllSettings}</button>
            {PRESETS.map((p) => <button key={p.id} className={preset === p.id ? "" : "secondary"} aria-pressed={preset === p.id} title={p.help} onClick={() => setPreset(p.id)}>{p.name}</button>)}
          </div>
          {activePreset && <p className="muted">{activePreset.help}</p>}
          {visibleColours.length > 0 && (
            <div className="row" role="group" aria-label={m.vsTintHeading(visibleColours.length)}>
              <label>{m.vsTintColor} <input type="color" value={tint.color} onChange={(e) => setTint({ ...tint, color: e.target.value })} /></label>
              <label>{m.vsTintStrength} <input type="range" min={0} max={1} step={0.05} value={tint.strength} onChange={(e) => setTint({ ...tint, strength: Number(e.target.value) })} /></label>
              <button className="secondary" onClick={applyTint}>{m.vsTintApply(visibleColours.length)}</button>
            </div>
          )}
          {!groups.length && <p className="muted">{m.vsNoMatch}</p>}
          <div className="vs-list">
            {groups.map(([name, g]) => {
              const isOpen = !!query || !!preset || changedOnly || openGroups.has(name);
              return (
                <div key={name}>
                  <button className="secondary vs-group" aria-expanded={isOpen} onClick={() => setOpenGroups((s) => { const n = new Set(s); if (n.has(name)) n.delete(name); else n.add(name); return n; })}>
                    {isOpen ? "v" : ">"} {name} <small>{m.vsGroupCount(g.entries.length + g.colours.length * 3, g.colours.length)}</small>
                  </button>
                  {isOpen && (
                    <div className="vs-body">
                      {g.colours.length > 0 && <div className="vs-colours">{g.colours.map((c) => {
                        const ch = c.keys.map((k) => parseNum(valueOf(k)));
                        const rgb = ch.map((x) => (x * 255) / c.scale) as [number, number, number];
                        const changed = c.keys.some((k) => pending.has(k));
                        return (
                          <label key={c.base} className={"vs-colour" + (changed ? " changed" : "")} title={c.keys.join("\n")}>
                            <input type="color" value={rgb.every(Number.isFinite) ? toHex(rgb) : "#000000"} onChange={(e) => { const [r, gg, b] = fromHex(e.target.value); const s = (x: number) => formatNum((x * c.scale) / 255, c.scale === 255); setVals({ [c.keys[0]]: s(r), [c.keys[1]]: s(gg), [c.keys[2]]: s(b) }); }} />
                            <span><b>{c.base.slice(name.length + 1) || c.base}</b> <small>{ch.map((x) => (c.scale === 255 ? Math.round(x) : x.toFixed(2))).join(" / ")}</small></span>
                          </label>
                        );
                      })}</div>}
                      {g.entries.map((e) => {
                        const v = valueOf(e.key), orig = base.get(e.key) ?? e.value, n = parseNum(v), numeric = Number.isFinite(n) && Number.isFinite(parseNum(orig));
                        const [lo, hi] = numeric ? numRange(e.key, parseNum(orig)) : [0, 1];
                        return (
                          <div key={e.key} className={"vs-row" + (v !== orig ? " changed" : "")}>
                            <span className="vs-name" title={m.vsRowTitle(e.key, e.line, e.repeats, orig)}>{e.key.slice(name.length + 1)}{e.repeats > 1 && <small> x{e.repeats}</small>}</span>
                            {numeric ? <input type="range" aria-label={e.key} min={Math.min(lo, n)} max={Math.max(hi, n)} step={hi - lo > 20 ? 1 : 0.001} value={n} onChange={(ev) => setVals({ [e.key]: ev.target.value })} /> : <span />}
                            <input type="text" aria-label={m.vsValueLabel(e.key)} key={v} defaultValue={v} onBlur={(ev) => { const t = ev.target.value.trim(); if (t && !/\s/.test(t) && t !== v) setVals({ [e.key]: t }); else ev.target.value = v; }} onKeyDown={(ev) => { if (ev.key === "Enter") (ev.target as HTMLInputElement).blur(); }} />
                            {v !== orig ? <button className="secondary" title={m.vsRevertTitle(orig)} onClick={() => setVals({ [e.key]: orig })}>{m.vsRevert}</button> : <span />}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
          <div className="row">
            <button disabled={busy || !pending.size} onClick={() => void queue()}>{busy ? m.vsSaving : m.vsQueue(pending.size)}</button>
            <button className="secondary" disabled={busy || !pending.size} onClick={() => { setPending(new Map()); setNote(""); }}>{m.vsDiscard}</button>
            <button className="secondary" disabled={busy} onClick={() => setOpen(false)}>{m.vsClose}</button>
          </div>
          <p className="muted">{m.vsNote}</p>
        </>
      )}
    </section>
  );
}
