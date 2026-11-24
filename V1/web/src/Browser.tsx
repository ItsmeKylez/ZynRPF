import { useEffect, useState } from "react";
import * as api from "./api";
import { Entry, VersionInfo } from "./api";
import { EntryDetail } from "./EntryDetail";
import { EditPanel, Target } from "./EditPanel";
import { EntryGrid, notes } from "./EntryGrid";
import { Summary } from "./Summary";
import { TreePick, TreeView } from "./TreeView";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";

const joinPath = (folder: string, name: string) => (folder === "/" ? `/${name}` : `${folder}/${name}`);

export function Browser({ versionId, workspaceId, versions, limits, onSaved, onChanged }: {
  versionId: string; workspaceId: string | null; versions: VersionInfo[];
  limits: { max_blob_bytes: number; max_versions: number }; onSaved: (v: string) => void; onChanged: () => void;
}) {
  const [parent, setParent] = useState("0");
  const [chain, setChain] = useState<{ id: string; name: string }[]>([{ id: "0", name: "" }]);
  const [selected, setSelected] = useState<Entry | null>(null);
  const [override, setOverride] = useState<Entry | null>(null);
  const [total, setTotal] = useState<number | null>(null);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<{ list: Entry[]; truncated: boolean } | null>(null);
  const [err, setErr] = useState<Shown | null>(null);
  const [collapse, setCollapse] = useState(0);
  const [sum, setSum] = useState<{ pending: number; after: number | null; stamp: string }>({ pending: 0, after: null, stamp: "0:" });
  const [picked, setPicked] = useState<{ path: string; name: string; kind: "directory" | "file" | "resource" } | null>(null);

  useEffect(() => { setParent("0"); setHits(null); setQ(""); setOverride(null); setPicked(null); }, [versionId]);
  useEffect(() => { if (sum.pending === 0) setPicked(null); }, [sum.pending]);
  useEffect(() => {
    let live = true;
    api.getPath(versionId, parent).then((r) => live && setChain(r.chain)).catch((e) => live && setErr(toShown(e)));
    return () => { live = false; };
  }, [versionId, parent]);

  const open = (e: Entry) => { setPicked(null); setOverride(null); setParent(e.id); };
  const doSearch = async (ev: React.FormEvent) => {
    ev.preventDefault(); setErr(null);
    if (!q.trim()) return;
    try { const r = await api.search(versionId, q.trim()); setHits({ list: r.hits, truncated: r.truncated }); }
    catch (e) { setErr(toShown(e)); }
  };
  const show = async (e: Entry) => {
    try {
      const p = await api.getPath(versionId, e.id);
      if (e.kind === "directory") { setOverride(null); setParent(e.id); }
      else { setParent(p.chain[Math.max(0, p.chain.length - 2)].id); setOverride(e); }
      setHits(null);
    } catch (x) { setErr(toShown(x)); }
  };
  const up = chain.length > 1 ? chain[chain.length - 2].id : null;
  const folderPath = chain.length > 1 ? "/" + chain.slice(1).map((c) => c.name).join("/") : "/";
  const sel = override || selected;
  const target: Target | null = picked ? picked : sel ? { path: sel.path || joinPath(folderPath, sel.name), name: sel.name, kind: sel.kind } : null;

  return (
    <>
    <div className="browser">
      <section className="panel" aria-labelledby="browse-h">
        <h2 id="browse-h">{m.browserHeading}</h2>
        <div className="row toolbar">
          <nav aria-label={m.breadcrumbLabel}>
            <ol className="crumbs">
              {chain.map((c, i) => (
                <li key={c.id}>{i === chain.length - 1
                  ? <span aria-current="page">{i === 0 ? m.rootName : c.name}</span>
                  : <button className="link" onClick={() => { setOverride(null); setParent(c.id); }}>{i === 0 ? m.rootName : c.name}</button>}</li>
              ))}
            </ol>
          </nav>
          <button className="secondary" disabled={up === null} onClick={() => up !== null && setParent(up)}>{m.goUp}</button>
          <a className="button" href={api.downloadUrl(versionId)}>{m.download}</a>
        </div>
        <form className="row" role="search" onSubmit={(e) => void doSearch(e)}>
          <label>{m.searchLabel} <input type="search" value={q} placeholder={m.searchPlaceholder} maxLength={100} onChange={(e) => setQ(e.target.value)} /></label>
          <button type="submit">{m.searchButton}</button>
          {hits && <button type="button" className="secondary" onClick={() => { setHits(null); setQ(""); }}>{m.clearSearch}</button>}
        </form>
        {err && <ErrorBox err={err} />}
        <h3>{m.treeHeading}</h3>
        <div className="row"><button className="secondary" onClick={() => setCollapse((c) => c + 1)}>{m.treeCollapseAll}</button></div>
        {sum.pending > 0 && <p className="muted">{m.treePreviewNote}</p>}
        <TreeView versionId={versionId} selectedPath={picked ? picked.path : (override || selected)?.path || null} collapseSignal={collapse}
          preview={workspaceId && sum.pending > 0 ? { workspaceId, stamp: sum.stamp } : null}
          onPick={(p: TreePick) => { if (p.entry) { setPicked(null); void show(p.entry); } else { setPicked({ path: p.path, name: p.name, kind: p.kind }); setOverride(null); setSelected(null); } }} />
        {total !== null && !hits && <p className="muted" aria-live="polite">{m.entryCount(total)}</p>}
        {hits ? (
          <div>
            <p aria-live="polite">{m.searchResults(hits.list.length)}</p>
            {hits.truncated && <p className="muted">{m.searchTruncated}</p>}
            {hits.list.length === 0 && <p>{m.noResults}</p>}
            <ul className="hits">
              {hits.list.map((h) => (
                <li key={h.id}>
                  <button className="link" onClick={() => void show(h)}>{h.path}</button>
                  <span className="muted"> {h.kind === "directory" ? "" : formatBytes(h.stored_size)} {notes(h)}</span>
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <EntryGrid versionId={versionId} parent={parent} onOpen={open} onTotal={setTotal} onSelect={(e) => { setSelected(e); if (e) { setOverride(null); setPicked(null); } }} />
        )}
      </section>
      <div className="side">
        <Summary version={versions.find((v) => v.id === versionId)} pending={sum.pending} afterBytes={sum.after} />
        <EntryDetail versionId={versionId} entry={override || selected} />
      </div>
    </div>
    {workspaceId && <EditPanel workspaceId={workspaceId} versionId={versionId} folderPath={folderPath} target={target} versions={versions} limits={limits} onSaved={onSaved} onChanged={onChanged} onSummary={(pending, after, stamp) => setSum({ pending, after, stamp })} />}
    </>
  );
}
