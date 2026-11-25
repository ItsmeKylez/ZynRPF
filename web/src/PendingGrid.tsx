import { useEffect, useState } from "react";
import * as api from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";

const PAGE = 200;
interface Row { name: string; isDir: boolean; size: number; isNew: boolean }

/** Read-only list of one folder as it will look after the pending edits are saved (F-06, grid part). Data comes from the server's replay of the edit log. */
export function PendingGrid({ workspaceId, versionId, path, stamp }: { workspaceId: string; versionId: string; path: string; stamp: string }) {
  const [rows, setRows] = useState<Row[] | null>(null);
  const [total, setTotal] = useState(0);
  const [err, setErr] = useState<Shown | null>(null);
  const [busy, setBusy] = useState(false);
  const load = async (cursor: number) => {
    setBusy(true);
    try {
      const r = await api.editTree(workspaceId, versionId, path, cursor, PAGE);
      const add = r.entries.map((e) => ({ name: e.name, isDir: e.kind === "directory", size: e.size, isNew: e.new }));
      setTotal(r.total); setRows((cur) => (cursor === 0 || !cur ? add : [...cur, ...add])); setErr(null);
    } catch (e) { setErr(toShown(e)); setRows(null); } finally { setBusy(false); }
  };
  useEffect(() => { setRows(null); void load(0); }, [workspaceId, versionId, path, stamp]); // eslint-disable-line react-hooks/exhaustive-deps
  if (err) return <div><p className="muted">{m.pendingFolderGone}</p><ErrorBox err={err} /></div>;
  if (!rows) return <p className="muted" role="status">{m.loading}</p>;
  return (
    <div className="pending-grid">
      <table aria-label={m.pendingGridLabel}>
        <thead><tr><th scope="col">{m.colName}</th><th scope="col">{m.colType}</th><th scope="col" className="num">{m.colSize}</th></tr></thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.name}>
              <td>{r.isDir ? "📁 " : ""}{r.name} {r.isNew && <span className="badge-new">{m.treeNewBadge}</span>}</td>
              <td>{r.isDir ? m.kindDirectory : m.kindFile}</td>
              <td className="num">{r.isDir ? "" : formatBytes(r.size)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {total === 0 && <p className="muted">{m.emptyFolder}</p>}
      {rows.length < total && <button className="link" disabled={busy} onClick={() => void load(rows.length)}>{m.treeLoadMore(rows.length, total)}</button>}
    </div>
  );
}
