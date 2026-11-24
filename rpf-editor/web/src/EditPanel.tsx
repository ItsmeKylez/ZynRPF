import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import { EditOp, EditState, Report, VersionInfo } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";

export interface Target { path: string; name: string; kind: "directory" | "file" | "resource" }
type Mode = null | "rename" | "mkdir" | "add" | "replace";
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const opText = (o: EditOp): string => {
  const size = formatBytes(o.size || 0);
  switch (o.type) {
    case "rename": return m.opRename(o.path || "", o.new_name || "");
    case "delete": return m.opDelete(o.path || "");
    case "mkdir": return m.opMkdir(o.parent || "/", o.name || "");
    case "add_file": return m.opAdd(o.parent || "/", o.name || "", size);
    default: return m.opReplace(o.path || "", size);
  }
};
const sevLabel = (s: string) => (s === "error" ? m.sevError : s === "warning" ? m.sevWarning : m.sevInfo);

export function ReportView({ report }: { report: Report }) {
  return (
    <div className="report" role="region" aria-label={m.reportHeading}>
      <h3>{m.reportHeading}</h3>
      <p role="status">{report.ok && report.warnings === 0 ? m.reportOk : m.reportProblems(report.errors, report.warnings)}</p>
      {report.planned && <p>{m.plannedSummary(report.planned.entries, formatBytes(report.planned.file_bytes))}</p>}
      {report.saved && <p>{m.savedSummary(report.saved.entries, formatBytes(report.saved.file_bytes), report.saved.copied_entries, report.saved.new_entries)}</p>}
      {report.findings.length > 0 && (
        <ul className="findings">
          {report.findings.map((f, i) => (
            <li key={i} className={"sev-" + f.severity}>
              <strong>{sevLabel(f.severity)}:</strong> {f.message}{f.detail ? ` (${f.detail})` : ""}
              <div className="muted">{f.next_step} {m.checkCodeLabel}: <code>{f.code}</code></div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function EditPanel({ workspaceId, versionId, folderPath, target, versions, limits, onSaved, onChanged }: {
  workspaceId: string; versionId: string; folderPath: string; target: Target | null; versions: VersionInfo[];
  limits: { max_blob_bytes: number; max_versions: number }; onSaved: (versionId: string) => void; onChanged: () => void;
}) {
  const [state, setState] = useState<EditState | null>(null);
  const [claimed, setClaimed] = useState<"checking" | "writer" | "readonly">("checking");
  const [err, setErr] = useState<Shown | null>(null);
  const [mode, setMode] = useState<Mode>(null);
  const [text, setText] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [uploadPct, setUploadPct] = useState<number | null>(null);
  const [savePct, setSavePct] = useState<number | null>(null);
  const [report, setReport] = useState<Report | null>(null);
  const [notice, setNotice] = useState("");
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const [vReport, setVReport] = useState<{ id: string; report: Report } | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const abort = useRef<AbortController | null>(null);

  // One writer per workspace: claim the lease, renew it, and keep checking while read-only (D-020).
  useEffect(() => {
    let live = true;
    const claim = async () => {
      try { const s = await api.claimWriter(workspaceId); if (live) { setState(s); setClaimed("writer"); } }
      catch (e) {
        if (!live) return;
        if (e instanceof api.ApiError && e.code === "writer_busy") {
          setClaimed("readonly");
          try { setState(await api.getEdit(workspaceId)); } catch { /* shown on next tick */ }
        } else setErr(toShown(e));
      }
    };
    void claim();
    const t = setInterval(() => void claim(), 10000);
    const bye = () => api.releaseWriter(workspaceId);
    window.addEventListener("pagehide", bye);
    return () => { live = false; clearInterval(t); window.removeEventListener("pagehide", bye); bye(); };
  }, [workspaceId]);

  const readOnly = claimed !== "writer";
  const pending = state?.cursor || 0;
  const isBusy = busy || savePct !== null || !!state?.saving;

  const run = async (f: () => Promise<EditState>) => {
    setErr(null); setNotice("");
    try { setState(await f()); }
    catch (e) {
      if (e instanceof api.ApiError && e.code === "writer_busy") setClaimed("readonly");
      setErr(toShown(e));
      throw e;
    }
  };
  const guarded = async (f: () => Promise<void>) => { setBusy(true); try { await f(); } catch { /* shown */ } finally { setBusy(false); } };

  const submit = (ev: React.FormEvent) => {
    ev.preventDefault();
    void guarded(async () => {
      if (mode === "rename" && target) await run(() => api.addOp(workspaceId, versionId, { op: "rename", path: target.path, new_name: text.trim() }));
      else if (mode === "mkdir") await run(() => api.addOp(workspaceId, versionId, { op: "mkdir", parent: folderPath, name: text.trim() }));
      else if ((mode === "add" || mode === "replace") && file) {
        if (file.size > limits.max_blob_bytes) {
          setErr({ message: m.contentTooLarge(formatBytes(file.size), formatBytes(limits.max_blob_bytes)), next: m.contentTooLargeNext });
          return;
        }
        abort.current = new AbortController();
        setUploadPct(0);
        let blob: { blob_id: number };
        try { blob = await api.putBlob(workspaceId, file, (l) => setUploadPct(Math.floor((l / Math.max(1, file.size)) * 100)), abort.current.signal); }
        catch (e) { setErr(toShown(e)); throw e; }
        finally { setUploadPct(null); }
        if (mode === "add") await run(() => api.addOp(workspaceId, versionId, { op: "add_file", parent: folderPath, name: text.trim(), blob_id: blob.blob_id }));
        else if (target) await run(() => api.addOp(workspaceId, versionId, { op: "replace", path: target.path, blob_id: blob.blob_id }));
      }
      setMode(null); setText(""); setFile(null); setReport(null);
    });
  };
  const open = (md: Mode) => { setMode(md); setText(md === "rename" && target ? target.name : ""); setFile(null); setErr(null); setNotice(""); };
  const direct = (f: () => Promise<EditState>) => void guarded(async () => { setReport(null); await run(f); });

  const check = () => void guarded(async () => {
    setErr(null); setNotice("");
    try { setReport(await api.checkEdits(workspaceId)); } catch (e) { setErr(toShown(e)); }
  });
  const save = () => void guarded(async () => {
    setErr(null); setNotice(""); setReport(null); setSavePct(0);
    try {
      const { job_id } = await api.saveEdits(workspaceId);
      let j = await api.getJob(job_id);
      while (j.state !== "succeeded" && j.state !== "failed") { setSavePct(j.progress); await sleep(300); j = await api.getJob(job_id); }
      setReport(j.report || null);
      if (j.state === "succeeded" && j.version_id) { setNotice(m.saveDone); setState(await api.getEdit(workspaceId)); onSaved(j.version_id); }
      else if (j.error) { setErr({ title: m.saveFailedTitle, message: j.error.message, next: j.error.next_step ? `${j.error.next_step} ${m.saveFailedNote}` : m.saveFailedNote, code: j.error.code }); setState(await api.getEdit(workspaceId)); }
    } catch (e) { setErr(toShown(e)); try { setState(await api.getEdit(workspaceId)); } catch { /* ignore */ } }
    finally { setSavePct(null); }
  });
  const discard = () => void guarded(async () => { setConfirmDiscard(false); setReport(null); await run(() => api.discardEdits(workspaceId)); });

  const checkVersion = useCallback((id: string) => void guarded(async () => {
    setErr(null);
    try { setVReport({ id, report: await api.checkVersion(id) }); } catch (e) { setErr(toShown(e)); }
  }), []); // eslint-disable-line react-hooks/exhaustive-deps
  const removeVersion = (id: string) => void guarded(async () => {
    setConfirmDelete(null); setErr(null);
    try { await api.deleteVersion(id); onChanged(); } catch (e) { setErr(toShown(e)); }
  });

  const canRename = !!target && target.path !== "/";
  const canReplace = !!target && target.kind === "file";
  const dis = readOnly || isBusy;
  const ready = versions.filter((v) => v.state === "ready");

  return (
    <section className="panel" aria-labelledby="edit-h">
      <h2 id="edit-h">{m.editHeading}</h2>
      <p className="muted">{m.editIntro}</p>
      {claimed === "checking" && <p role="status">{m.claiming}</p>}
      {claimed === "readonly" && <p role="status" className="notice">{m.readOnlyTab}</p>}
      <p>{target ? m.targetLabel(target.path) : m.noTarget} <span className="muted">{m.inFolder(folderPath)}</span></p>
      <div className="row" role="toolbar" aria-label={m.toolbarLabel}>
        <button disabled={dis || !canRename} onClick={() => open("rename")}>{m.btnRename}</button>
        <button disabled={dis || !canRename} onClick={() => target && direct(() => api.addOp(workspaceId, versionId, { op: "delete", path: target.path }))}>{m.btnDelete}</button>
        <button disabled={dis} onClick={() => open("mkdir")}>{m.btnNewFolder}</button>
        <button disabled={dis} onClick={() => open("add")}>{m.btnAddFile}</button>
        <button disabled={dis || !canReplace} onClick={() => open("replace")}>{m.btnReplace}</button>
        <button className="secondary" disabled={dis || !state?.can_undo} onClick={() => direct(() => api.undoOp(workspaceId))}>{m.btnUndo}</button>
        <button className="secondary" disabled={dis || !state?.can_redo} onClick={() => direct(() => api.redoOp(workspaceId))}>{m.btnRedo}</button>
      </div>
      {target && target.kind === "directory" && <p className="muted">{m.replaceNotForFolders}</p>}
      {target && target.kind === "resource" && <p className="muted">{m.replaceNotForResources}</p>}

      {mode && (
        <form className="row" onSubmit={submit}>
          {(mode === "add" || mode === "replace") && (
            <label>{mode === "add" ? m.chooseContentLabel : m.chooseReplacementLabel}{" "}
              <input type="file" onChange={(e) => { const f = e.target.files?.[0] || null; setFile(f); if (f && mode === "add" && !text) setText(f.name); }} />
            </label>
          )}
          {mode !== "replace" && (
            <label>{mode === "rename" ? m.newNameLabel : mode === "mkdir" ? m.folderNameLabel : m.fileNameLabel}{" "}
              <input type="text" value={text} maxLength={255} onChange={(e) => setText(e.target.value)} autoFocus={mode !== "add"} />
            </label>
          )}
          <button type="submit" disabled={dis || (mode !== "replace" && !text.trim()) || ((mode === "add" || mode === "replace") && !file)}>{m.btnApply}</button>
          <button type="button" className="secondary" onClick={() => { abort.current?.abort(); setMode(null); }}>{m.cancel}</button>
          {(mode === "add" || mode === "replace") && <span className="muted">{m.storedOnlyNote}</span>}
        </form>
      )}
      {uploadPct !== null && <p><progress aria-label={m.uploadingContent(uploadPct)} max={100} value={uploadPct} /> {m.uploadingContent(uploadPct)}</p>}
      {err && <ErrorBox err={err} />}

      <h3>{m.changesHeading}</h3>
      {state && state.ops.length > 0 ? (
        <ol className="changes" aria-label={m.changesLabel}>
          {state.ops.map((o) => <li key={o.seq} className={o.active ? "" : "muted undone"}>{opText(o)}{o.active ? "" : ` (${m.undone})`}</li>)}
        </ol>
      ) : <p className="muted">{m.noChanges}</p>}
      <div className="row">
        <button className="secondary" disabled={dis || pending === 0} onClick={check}>{m.btnCheck}</button>
        <button disabled={dis || pending === 0 || ready.length >= limits.max_versions} onClick={save}>{m.btnSave}</button>
        {!confirmDiscard
          ? <button className="secondary" disabled={dis || !state || state.ops.length === 0} onClick={() => setConfirmDiscard(true)}>{m.btnDiscard}</button>
          : <span role="alertdialog" aria-label={m.btnDiscard} className="row">{m.discardConfirm}
              <button className="danger" onClick={discard}>{m.discardYes}</button>
              <button className="secondary" onClick={() => setConfirmDiscard(false)}>{m.cancel}</button></span>}
      </div>
      {savePct !== null && <p><progress aria-label={m.saveProgressLabel} max={100} value={savePct} /> {m.saving(savePct)}</p>}
      {notice && <p role="status">{notice}</p>}
      {report && <ReportView report={report} />}

      <h3>{m.versionsHeading}</h3>
      <p className="muted">{m.versionLimit(ready.length, limits.max_versions)}</p>
      <ul className="versions" aria-label={m.versionsLabel}>
        {ready.map((v) => (
          <li key={v.id} className="row">
            <span>{m.versionOption(v.filename, v.entry_count)} - {formatBytes(v.size)} - {m.versionCreatedBy(v.origin || "upload")}{v.id === versionId ? "" : ""}</span>
            <a className="button" href={api.downloadUrl(v.id)}>{m.download}</a>
            <button className="secondary" disabled={isBusy} onClick={() => checkVersion(v.id)}>{m.btnCheckVersion}</button>
            {ready.length > 1 && (confirmDelete === v.id
              ? <span role="alertdialog" aria-label={m.btnDeleteVersion} className="row">{m.versionDeleteConfirm}
                  <button className="danger" onClick={() => removeVersion(v.id)}>{m.btnDeleteVersion}</button>
                  <button className="secondary" onClick={() => setConfirmDelete(null)}>{m.cancel}</button></span>
              : <button className="secondary" disabled={isBusy} onClick={() => setConfirmDelete(v.id)}>{m.btnDeleteVersion}</button>)}
          </li>
        ))}
      </ul>
      {vReport && <ReportView report={vReport.report} />}
    </section>
  );
}
