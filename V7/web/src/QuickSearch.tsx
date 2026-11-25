import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { Entry } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { m } from "./messages";

/** Quick search dialog (Ctrl+P): type to search the whole open archive, press Enter or click to jump to a result. */
export function QuickSearch({ versionId, onPick, onClose }: { versionId: string; onPick: (e: Entry) => void; onClose: () => void }) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<Entry[]>([]);
  const [truncated, setTruncated] = useState(false);
  const [err, setErr] = useState<Shown | null>(null);
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const opener = useRef<Element | null>(document.activeElement);
  useEffect(() => { input.current?.focus(); const o = opener.current; return () => { if (o instanceof HTMLElement) o.focus(); }; }, []);
  useEffect(() => {
    const text = q.trim();
    if (!text) { setHits([]); setTruncated(false); return; }
    let alive = true;
    const t = setTimeout(async () => {
      try { const r = await api.search(versionId, text); if (alive) { setHits(r.hits); setTruncated(r.truncated); setActive(0); setErr(null); } }
      catch (e) { if (alive) setErr(toShown(e)); }
    }, 200);
    return () => { alive = false; clearTimeout(t); };
  }, [q, versionId]);
  const pick = (e: Entry | undefined) => { if (e) { onPick(e); onClose(); } };
  const key = (ev: React.KeyboardEvent) => {
    if (ev.key === "Escape") { ev.preventDefault(); onClose(); }
    else if (ev.key === "ArrowDown") { ev.preventDefault(); setActive((a) => Math.min(hits.length - 1, a + 1)); }
    else if (ev.key === "ArrowUp") { ev.preventDefault(); setActive((a) => Math.max(0, a - 1)); }
    else if (ev.key === "Enter") { ev.preventDefault(); pick(hits[active]); }
    else if (ev.key === "Tab") { ev.preventDefault(); input.current?.focus(); } // keep focus inside the dialog
  };
  return (
    <div className="modal-back" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div role="dialog" aria-modal="true" aria-label={m.quickTitle} className="modal quick" onKeyDown={key}>
        <h2>{m.quickTitle}</h2>
        <input ref={input} type="search" aria-label={m.quickInputLabel} placeholder={m.quickPlaceholder} maxLength={100} value={q}
          role="combobox" aria-expanded={hits.length > 0} aria-controls="quick-list" aria-activedescendant={hits[active] ? `qs-${hits[active].id}` : undefined}
          onChange={(e) => setQ(e.target.value)} />
        {err && <ErrorBox err={err} />}
        <ul id="quick-list" role="listbox" aria-label={m.quickResults}>
          {hits.map((h, i) => (
            <li key={h.id} id={`qs-${h.id}`} role="option" aria-selected={i === active} className={i === active ? "active" : ""} onMouseDown={(e) => { e.preventDefault(); pick(h); }}>
              <span>{h.name}</span> <span className="muted">{h.path}</span>
            </li>
          ))}
        </ul>
        <p className="muted" aria-live="polite">{q.trim() ? (hits.length === 0 ? m.quickNone : m.searchResults(hits.length)) : m.quickHint}{truncated ? " " + m.searchTruncated : ""}</p>
      </div>
    </div>
  );
}
