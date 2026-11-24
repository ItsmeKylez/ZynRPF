import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import { Entry } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";

const ROW = 32, PAGE = 500, OVERSCAN = 8;
type Sort = "name" | "size" | "kind";
const kindLabel = (k: string) => (k === "directory" ? m.kindDirectory : k === "resource" ? m.kindResource : m.kindFile);
export const notes = (e: Entry) => [e.compressed && m.noteCompressed, e.encryption_tag !== null && e.encryption_tag !== 0 && m.noteEncrypted, e.large_resource && m.noteLarge].filter(Boolean).join(", ");

export function EntryGrid({ versionId, parent, onOpen, onSelect, onTotal }: {
  versionId: string; parent: string; onOpen: (e: Entry) => void; onSelect: (e: Entry | null) => void; onTotal: (n: number) => void;
}) {
  const [total, setTotal] = useState<number | null>(null);
  const [sort, setSort] = useState<Sort>("name");
  const [order, setOrder] = useState<"asc" | "desc">("asc");
  const [, force] = useState(0);
  const [active, setActive] = useState(0);
  const [err, setErr] = useState<Shown | null>(null);
  const pages = useRef(new Map<number, Entry[]>());
  const pending = useRef(new Set<number>());
  const scroller = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ top: 0, h: 400 });
  const gen = useRef(0);

  const loadPage = useCallback(async (p: number) => {
    if (pages.current.has(p) || pending.current.has(p)) return;
    pending.current.add(p);
    const g = gen.current;
    try {
      const r = await api.listEntries(versionId, { parent, cursor: p * PAGE, limit: PAGE, sort, order });
      if (g !== gen.current) return;
      pages.current.set(p, r.entries);
      setTotal(r.total); onTotal(r.total);
      force((x) => x + 1);
    } catch (e) { if (g === gen.current) setErr(toShown(e)); }
    finally { pending.current.delete(p); }
  }, [versionId, parent, sort, order, onTotal]);

  useEffect(() => {
    gen.current++; pages.current = new Map(); pending.current = new Set();
    setTotal(null); setErr(null); setActive(0); onSelect(null);
    if (scroller.current) scroller.current.scrollTop = 0;
    void loadPage(0);
  }, [versionId, parent, sort, order]); // eslint-disable-line react-hooks/exhaustive-deps

  const at = (i: number): Entry | undefined => pages.current.get(Math.floor(i / PAGE))?.[i % PAGE];
  const n = total ?? 0;
  const first = Math.max(0, Math.floor(view.top / ROW) - OVERSCAN);
  const last = Math.min(n - 1, Math.ceil((view.top + view.h) / ROW) + OVERSCAN);
  useEffect(() => { for (let p = Math.floor(first / PAGE); p <= Math.floor(Math.max(first, last) / PAGE); p++) if (total !== null) void loadPage(p); }, [first, last, total, loadPage]);
  useEffect(() => { const e = at(active); if (e) onSelect(e); }, [active, total, pages.current.size]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    const el = scroller.current; if (!el) return;
    const ro = new ResizeObserver(() => setView((v) => ({ ...v, h: el.clientHeight })));
    ro.observe(el); setView({ top: el.scrollTop, h: el.clientHeight });
    return () => ro.disconnect();
  }, []);

  const reveal = (i: number) => {
    const el = scroller.current; if (!el) return;
    if (i * ROW < el.scrollTop) el.scrollTop = i * ROW;
    else if ((i + 1) * ROW > el.scrollTop + el.clientHeight) el.scrollTop = (i + 1) * ROW - el.clientHeight;
  };
  const move = (i: number) => { const j = Math.max(0, Math.min(n - 1, i)); setActive(j); reveal(j); };
  const onKey = (ev: React.KeyboardEvent) => {
    const page = Math.max(1, Math.floor(view.h / ROW) - 1);
    const keys: Record<string, () => void> = {
      ArrowDown: () => move(active + 1), ArrowUp: () => move(active - 1), PageDown: () => move(active + page), PageUp: () => move(active - page),
      Home: () => move(0), End: () => move(n - 1),
      Enter: () => { const e = at(active); if (e?.kind === "directory") onOpen(e); },
    };
    const f = keys[ev.key]; if (f) { ev.preventDefault(); f(); }
  };
  const th = (col: Sort, label: string) => (
    <button className="th" aria-label={m.sortBy(label)} onClick={() => { if (sort === col) setOrder(order === "asc" ? "desc" : "asc"); else { setSort(col); setOrder("asc"); } }}>
      {label}{sort === col ? (order === "asc" ? " ▲" : " ▼") : ""}
    </button>
  );

  if (err) return <ErrorBox err={err} onRetry={() => { setErr(null); gen.current++; pages.current = new Map(); void loadPage(0); }} />;
  const rows = [];
  for (let i = first; i <= last; i++) {
    const e = at(i);
    rows.push(
      <div key={i} id={`row-${i}`} role="row" aria-rowindex={i + 2} aria-selected={i === active} className={"row-item" + (i === active ? " active" : "")}
        style={{ top: i * ROW, height: ROW }}
        onClick={() => { setActive(i); }} onDoubleClick={() => { if (e?.kind === "directory") onOpen(e); }}>
        {e ? (<>
          <div role="gridcell" className="c-name">{e.kind === "directory" ? "📁 " : ""}{e.name}</div>
          <div role="gridcell">{kindLabel(e.kind)}</div>
          <div role="gridcell" className="num">{e.kind === "directory" ? m.entryCount(e.child_count ?? 0) : formatBytes(e.stored_size)}</div>
          <div role="gridcell">{notes(e)}</div>
        </>) : <div role="gridcell" className="muted">{m.rowLoading}</div>}
      </div>
    );
  }
  return (
    <div role="grid" aria-label={m.gridLabel} aria-rowcount={n + 1} aria-colcount={4} tabIndex={0} aria-activedescendant={n ? `row-${active}` : undefined} onKeyDown={onKey} className="grid">
      <div role="row" aria-rowindex={1} className="head">
        <div role="columnheader">{th("name", m.colName)}</div><div role="columnheader">{th("kind", m.colType)}</div>
        <div role="columnheader" className="num">{th("size", m.colSize)}</div><div role="columnheader">{m.colFlags}</div>
      </div>
      <div className="scroller" ref={scroller} onScroll={(e) => setView({ top: e.currentTarget.scrollTop, h: e.currentTarget.clientHeight })}>
        {total === 0 && <p className="muted pad">{m.emptyFolder}</p>}
        <div className="spacer" style={{ height: n * ROW }}>{rows}</div>
      </div>
    </div>
  );
}
