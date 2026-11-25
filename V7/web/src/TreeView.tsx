import { useCallback, useEffect, useState } from "react";
import * as api from "./api";
import { Entry } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";

const PAGE = 200;
export interface PreviewCfg { workspaceId: string; stamp: string }
export interface TreePick { path: string; name: string; kind: "directory" | "file" | "resource"; entry: Entry | null }
interface Row { key: string; name: string; isDir: boolean; size: number; isNew: boolean; id: string | null; entry: Entry | null }
interface Node { rows: Row[]; total: number }
const join = (p: string, n: string) => (p === "/" ? `/${n}` : `${p}/${n}`);

/**
 * Lazy tree: a folder's children are requested only when it is expanded, 200 at a time (works for very large archives).
 * With `preview` set, folders are listed from the server's replay of the pending edits, so renames, deletes and
 * new items show before saving (F-06).
 */
export function TreeView({ versionId, selectedPath, onPick, collapseSignal, preview, expandTo }: {
  versionId: string; selectedPath: string | null; onPick: (p: TreePick) => void; collapseSignal: number; preview: PreviewCfg | null; expandTo: number;
}) {
  return (
    <div role="tree" aria-label={m.treeLabel} className="tree">
      <Level key={`${versionId}:${collapseSignal}:${preview ? preview.stamp : "base"}`} versionId={versionId} parentId="0" path="/" depth={0}
        selectedPath={selectedPath} onPick={onPick} preview={preview} expandTo={expandTo} />
    </div>
  );
}

function Level({ versionId, parentId, path, depth, selectedPath, onPick, preview, expandTo }: {
  versionId: string; parentId: string; path: string; depth: number; selectedPath: string | null; onPick: (p: TreePick) => void; preview: PreviewCfg | null; expandTo: number;
}) {
  const [node, setNode] = useState<Node | null>(null);
  const [err, setErr] = useState<Shown | null>(null);
  const [busy, setBusy] = useState(false);
  const load = useCallback(async (cursor: number) => {
    setBusy(true);
    try {
      let rows: Row[]; let total: number;
      if (preview) {
        const r = await api.editTree(preview.workspaceId, versionId, path, cursor, PAGE);
        total = r.total;
        rows = r.entries.map((e) => ({ key: e.name, name: e.name, isDir: e.kind === "directory", size: e.size, isNew: e.new, id: null, entry: null }));
      } else {
        const r = await api.listEntries(versionId, { parent: parentId, cursor, limit: PAGE, sort: "name", order: "asc" });
        total = r.total;
        rows = r.entries.map((e) => ({ key: e.id, name: e.name, isDir: e.kind === "directory", size: e.stored_size, isNew: false, id: e.id, entry: e }));
      }
      setNode((cur) => ({ rows: cursor === 0 || !cur ? rows : [...cur.rows, ...rows], total }));
    } catch (e) { setErr(toShown(e)); } finally { setBusy(false); }
  }, [versionId, parentId, path, preview]);
  useEffect(() => { void load(0); }, [load]);
  if (err) return <ErrorBox err={err} />;
  if (!node) return <p className="muted" role="status">{m.loading}</p>;
  if (node.total === 0) return <p className="muted tree-empty" style={{ paddingLeft: depth * 16 + 8 }}>{m.treeEmptyFolder}</p>;
  return (
    <ul role="group" className="tree-level">
      {node.rows.map((r) => <Item key={r.key} row={r} versionId={versionId} path={path} depth={depth} selectedPath={selectedPath} onPick={onPick} preview={preview} expandTo={expandTo} />)}
      {node.rows.length < node.total && (
        <li role="none"><button className="link" disabled={busy} style={{ marginLeft: depth * 16 + 8 }} onClick={() => void load(node.rows.length)}>{m.treeLoadMore(node.rows.length, node.total)}</button></li>
      )}
    </ul>
  );
}

function Item({ row, versionId, path, depth, selectedPath, onPick, preview, expandTo }: {
  row: Row; versionId: string; path: string; depth: number; selectedPath: string | null; onPick: (p: TreePick) => void; preview: PreviewCfg | null; expandTo: number;
}) {
  const [open, setOpen] = useState(row.isDir && depth < expandTo);
  const full = row.entry?.path || join(path, row.name);
  const kind: TreePick["kind"] = row.isDir ? "directory" : row.entry ? row.entry.kind : "file";
  return (
    <li role="treeitem" aria-expanded={row.isDir ? open : undefined} aria-selected={selectedPath === full}>
      <div className={"tree-row" + (selectedPath === full ? " active" : "")} style={{ paddingLeft: depth * 16 + 4 }}>
        {row.isDir
          ? <button className="twisty" aria-label={open ? m.treeCollapse(row.name) : m.treeExpand(row.name)} onClick={() => setOpen(!open)}>{open ? "▼" : "▶"}</button>
          : <span className="twisty-gap" />}
        <button className="link tree-name" onClick={() => { onPick({ path: full, name: row.name, kind, entry: row.entry }); if (row.isDir) setOpen(true); }}>{row.name}</button>
        {row.isNew && <span className="badge-new">{m.treeNewBadge}</span>}
        {!row.isDir && <span className="muted tree-size">{formatBytes(row.size)}</span>}
      </div>
      {row.isDir && open && <Level versionId={versionId} parentId={row.id || "0"} path={full} depth={depth + 1} selectedPath={selectedPath} onPick={onPick} preview={preview} expandTo={expandTo} />}
    </li>
  );
}
