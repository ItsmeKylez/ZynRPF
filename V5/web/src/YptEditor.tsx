import { useEffect, useMemo, useState } from "react";
import * as api from "./api";
import { Entry, YptGroup, YptFrame } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";
import { applyBulk, BulkSettings, changeLines, filterGroups, isColour, isSize, Value } from "./yptbulk";

// Particle editor panel. Layout and bulk-edit behaviour follow the original Zyn-RPF (GPL-3.0); see LICENSE-GPL-NOTICE.md.
const LABELS: Record<string, string> = {
  "ptxu_Colour:m_rgbaMinKFP": "Color (min)", "ptxu_Colour:m_rgbaMaxKFP": "Color (max)", "ptxu_Colour:m_emissiveIntensityKFP": "Glow intensity",
  "ptxu_Size:m_whdMinKFP": "Size (min)", "ptxu_Size:m_whdMaxKFP": "Size (max)",
  "ptxu_Light:m_rgbMinKFP": "Light color (min)", "ptxu_Light:m_rgbMaxKFP": "Light color (max)", "ptxu_Light:m_intensityKFP": "Light intensity", "ptxu_Light:m_rangeKFP": "Light range",
  "ptxu_Light:m_coronaRgbMinKFP": "Corona color (min)", "ptxu_Light:m_coronaRgbMaxKFP": "Corona color (max)", "ptxu_Light:m_coronaIntensityKFP": "Corona intensity", "ptxu_Light:m_coronaSizeKFP": "Corona size",
  "ptxEffectRule:m_colourTintMinKFP": "Effect tint (min)", "ptxEffectRule:m_colourTintMaxKFP": "Effect tint (max)", "ptxEffectRule:m_zoomScalarKFP": "Effect scale",
  "ptxEmitterRule:m_spawnRateOverTimeKFP": "Spawn rate (over time)", "ptxEmitterRule:m_spawnRateOverDistKFP": "Spawn rate (over distance)", "ptxEmitterRule:m_particleLifeKFP": "Particle life",
  "ptxEmitterRule:m_sizeScalarKFP": "Size multiplier", "ptxEmitterRule:m_speedScalarKFP": "Speed multiplier", "ptxEmitterRule:m_playbackRateScalarKFP": "Playback rate",
  "ptxu_Decal:m_alphaKFP": "Decal opacity", "ptxu_Decal:m_dimensionsKFP": "Decal size",
};
const KIND_LABEL: Record<YptGroup["kind"], string> = { effect: "effect", emitter: "emitter", particle: "particle" };
type Kind = "all" | YptGroup["kind"];

const toHex = (d: number[]) => "#" + d.slice(0, 3).map((x) => Math.round(Math.min(1, Math.max(0, x)) * 255).toString(16).padStart(2, "0")).join("");
const fromHex = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);

/** Edits the keyframes of a particle dictionary (.ypt): loads them from the server, lets the person change values (one by one or in bulk) and queues a Replace operation. */
export function YptEditor({ workspaceId, versionId, entry, path, maxBlob, onQueued }: {
  workspaceId: string; versionId: string; entry: Entry; path: string; maxBlob: number; onQueued: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<Shown | null>(null);
  const [groups, setGroups] = useState<YptGroup[] | null>(null);
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<Kind>("all");
  const [coloursOnly, setColoursOnly] = useState(false);
  const [pending, setPending] = useState<Map<number, Value>>(new Map());
  const [shown, setShown] = useState(60);
  const [note, setNote] = useState("");
  const [bulk, setBulk] = useState({ tint: "#e5243b", strength: 0, hue: 0, size: 1, light: 1, density: 1 });
  useEffect(() => { setOpen(false); setErr(null); setGroups(null); setPending(new Map()); setNote(""); setQuery(""); setShown(60); }, [versionId, entry.id]);

  const filtered = useMemo(() => filterGroups(groups ?? [], query, kind).filter((g) => !coloursOnly || g.properties.some((p) => isColour(p.name))), [groups, query, kind, coloursOnly]);
  const counts = useMemo(() => {
    const c = { effect: 0, emitter: 0, particle: 0 };
    for (const g of groups ?? []) c[g.kind]++;
    return c;
  }, [groups]);

  if (entry.large_resource) return null;

  const load = async () => {
    setBusy(true); setErr(null); setNote("");
    try {
      const r = await api.yptRead(versionId, entry.id);
      setGroups(r.groups); setPending(new Map()); setOpen(true);
    } catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  };
  const valueOf = (f: YptFrame): Value => pending.get(f.offset) ?? f.value;
  const setValue = (f: YptFrame, v: Value) => setPending((p) => new Map(p).set(f.offset, v));
  const runBulk = () => {
    const next = new Map(pending);
    const s: BulkSettings = { hue: bulk.hue, tint: bulk.strength > 0 ? bulk.tint : undefined, strength: bulk.strength, size: bulk.size, light: bulk.light, density: bulk.density };
    const n = applyBulk(filtered, s, next);
    setPending(next);
    setBulk({ ...bulk, strength: 0, hue: 0, size: 1, light: 1, density: 1 });
    setNote(n ? m.yptBulkChanged(n) : m.yptBulkNothing);
  };
  const queue = async () => {
    setBusy(true); setErr(null);
    try {
      const bytes = await api.yptEdit(versionId, entry.id, changeLines(pending));
      if (bytes.length > maxBlob) { setErr({ message: m.contentTooLarge(formatBytes(bytes.length), formatBytes(maxBlob)), next: m.contentTooLargeNext }); return; }
      const blob = await api.putBlob(workspaceId, new Blob([bytes as BlobPart]), () => {}, new AbortController().signal);
      await api.addOp(workspaceId, versionId, { op: "replace", path, blob_id: blob.blob_id });
      setPending(new Map()); setNote(m.yptQueued); onQueued();
    } catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  };

  return (
    <section className="panel" aria-labelledby="ypt-h">
      <h2 id="ypt-h">{m.yptHeading}</h2>
      {!open && <button disabled={busy} onClick={() => void load()}>{busy ? m.yptLoading : m.yptOpen}</button>}
      {err && <ErrorBox err={err} />}
      {open && groups && (
        <>
          <p className="muted">{m.yptSummary(counts.effect, counts.emitter, counts.particle)}</p>
          {note && <p role="status">{note}</p>}
          <div className="ypt-toolbar">
            <input type="search" aria-label={m.yptSearchLabel} placeholder={m.yptSearchPlaceholder} value={query} onChange={(e) => { setQuery(e.target.value); setShown(60); }} />
            {(["all", "effect", "emitter", "particle"] as const).map((k) => (
              <button key={k} className={kind === k ? "" : "secondary"} aria-pressed={kind === k} onClick={() => { setKind(k); setShown(60); }}>
                {k === "all" ? m.yptKindAll : k === "effect" ? m.yptKindEffect : k === "emitter" ? m.yptKindEmitter : m.yptKindParticle}
              </button>
            ))}
            <label><input type="checkbox" checked={coloursOnly} onChange={(e) => setColoursOnly(e.target.checked)} /> {m.yptColoursOnly}</label>
          </div>
          <div className="ypt-layout">
            <div className="ypt-list">
              {!filtered.length && <p className="muted">{m.yptNoMatch}</p>}
              {filtered.slice(0, shown).map((g, gi) => (
                <div key={`${g.kind}|${g.name}|${g.behaviour}|${gi}`} className="ypt-card">
                  <h3>{g.name || m.yptUnnamed} <span className="badge-new">{KIND_LABEL[g.kind]}</span> {g.behaviour && <span className="badge-new">{g.behaviour}</span>}</h3>
                  {g.properties.map((p) => (
                    <div key={p.name} className="ypt-prop">
                      <span className="ypt-name" title={p.name}>{LABELS[p.name] ?? p.name.split(":").pop()}</span>
                      <span className="ypt-frames">
                        {p.frames.map((f) => {
                          const v = valueOf(f);
                          const label = `${g.name} ${LABELS[p.name] ?? p.name} ${m.yptFrameTime(f.time[0].toFixed(2))}`;
                          return (
                            <span key={f.offset} className={`ypt-frame${pending.has(f.offset) ? " changed" : ""}`}>
                              <small>{f.time[0].toFixed(2)}</small>
                              {isColour(p.name) ? (
                                <>
                                  <input type="color" aria-label={label} value={toHex(v)} onChange={(e) => setValue(f, [...fromHex(e.target.value), v[3]] as Value)} />
                                  {/rgba|colourTint/i.test(p.name) && <input type="number" step={0.05} aria-label={`${label} ${m.yptAlpha}`} value={Number(v[3].toFixed(3))} onChange={(e) => setValue(f, [v[0], v[1], v[2], Number(e.target.value)])} />}
                                </>
                              ) : (
                                v.slice(0, isSize(p.name) ? 3 : 1).map((x, j) => (
                                  <input key={j} type="number" step={0.05} aria-label={`${label} ${j + 1}`} value={Number(x.toFixed(4))} onChange={(e) => setValue(f, v.map((y, t) => (t === j ? Number(e.target.value) : y)) as Value)} />
                                ))
                              )}
                            </span>
                          );
                        })}
                      </span>
                    </div>
                  ))}
                </div>
              ))}
              {filtered.length > shown && <button className="secondary" onClick={() => setShown(shown + 120)}>{m.yptMore(filtered.length - shown)}</button>}
            </div>
            <div className="ypt-side">
              <h3>{m.yptBulkHeading(filtered.length)}</h3>
              <p className="muted">{m.yptBulkHelp}</p>
              <label>{m.yptHue} <input type="range" min={-180} max={180} step={1} value={bulk.hue} onChange={(e) => setBulk({ ...bulk, hue: Number(e.target.value) })} /> {bulk.hue}</label>
              <label>{m.yptTint} <input type="color" value={bulk.tint} onChange={(e) => setBulk({ ...bulk, tint: e.target.value, strength: bulk.strength || 1 })} /></label>
              <label>{m.yptTintStrength} <input type="range" min={0} max={1} step={0.01} value={bulk.strength} onChange={(e) => setBulk({ ...bulk, strength: Number(e.target.value) })} /> {bulk.strength}</label>
              <label>{m.yptSize} <input type="range" min={0.1} max={4} step={0.05} value={bulk.size} onChange={(e) => setBulk({ ...bulk, size: Number(e.target.value) })} /> {bulk.size}</label>
              <label>{m.yptLight} <input type="range" min={0} max={5} step={0.05} value={bulk.light} onChange={(e) => setBulk({ ...bulk, light: Number(e.target.value) })} /> {bulk.light}</label>
              <label>{m.yptDensity} <input type="range" min={0} max={4} step={0.05} value={bulk.density} onChange={(e) => setBulk({ ...bulk, density: Number(e.target.value) })} /> {bulk.density}</label>
              <p><button disabled={!filtered.length} onClick={runBulk}>{m.yptApplyBulk}</button></p>
              <p className="muted">{m.yptDensityNote}</p>
              <h3>{m.yptHowHeading}</h3>
              <p className="muted">{m.yptHow}</p>
            </div>
          </div>
          <p>
            <button disabled={busy || pending.size === 0} onClick={() => void queue()}>{m.yptQueue(pending.size)}</button>{" "}
            <button className="secondary" disabled={pending.size === 0} onClick={() => setPending(new Map())}>{m.yptDiscard}</button>
          </p>
        </>
      )}
    </section>
  );
}
