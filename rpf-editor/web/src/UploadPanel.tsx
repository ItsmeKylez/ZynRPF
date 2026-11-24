import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import { ApiError, Limits, Workspace } from "./api";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";

type Phase = "idle" | "hashing" | "uploading" | "paused" | "processing" | "failed";
interface Saved { uploadId: string; name: string; size: number; lastModified: number; workspaceId: string }
const KEY = "rpf.upload";
const loadSaved = (): Saved | null => { try { return JSON.parse(localStorage.getItem(KEY) || "null"); } catch { return null; } };
const store = (s: Saved | null) => { try { s ? localStorage.setItem(KEY, JSON.stringify(s)) : localStorage.removeItem(KEY); } catch { /* storage unavailable */ } };
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const MAX_HASH_BYTES = 256 * 1024 * 1024;

async function sha256Hex(file: File): Promise<string> {
  const d = await crypto.subtle.digest("SHA-256", await file.arrayBuffer());
  return Array.from(new Uint8Array(d)).map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function UploadPanel({ limits, workspace, onWorkspace, onReady, onSessionLost }: {
  limits: Limits; workspace: Workspace | null; onWorkspace: (w: Workspace) => void; onReady: (versionId: string) => void; onSessionLost: () => void;
}) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [pct, setPct] = useState(0);
  const [jobPct, setJobPct] = useState(0);
  const [file, setFile] = useState<File | null>(null);
  const [err, setErr] = useState<Shown | null>(null);
  const [verify, setVerify] = useState(false);
  const abort = useRef<AbortController | null>(null);
  const uploadId = useRef<string | null>(null);
  const wsId = useRef<string | null>(null);
  const live = useRef(true);
  useEffect(() => () => { live.current = false; abort.current?.abort(); }, []);

  const fail = useCallback((e: unknown, title: string) => {
    if (e instanceof ApiError && e.code === "unauthenticated") { onSessionLost(); return; }
    setErr(toShown(e, title)); setPhase("failed");
  }, [onSessionLost]);

  const pollJob = useCallback(async (jobId: string) => {
    setPhase("processing"); setJobPct(0);
    for (;;) {
      await sleep(400);
      if (!live.current) return;
      const j = await api.getJob(jobId);
      setJobPct(j.progress);
      if (j.state === "succeeded" && j.version_id) { store(null); onReady(j.version_id); setPhase("idle"); setFile(null); return; }
      if (j.state === "failed") {
        store(null);
        setErr({ title: m.jobFailedTitle, message: j.error?.message || m.networkError, next: j.error?.next_step, code: j.error?.code });
        setPhase("failed"); return;
      }
    }
  }, [onReady]);

  const sendAll = useCallback(async (f: File, id: string) => {
    const ctl = new AbortController(); abort.current = ctl;
    setPhase("uploading"); setErr(null);
    try {
      const st = await api.uploadStatus(id);
      const received = new Set(st.received);
      const chunk = st.chunk_size, count = Math.ceil(f.size / chunk);
      let done = received.size * chunk;
      for (let i = 0; i < count; i++) {
        if (received.has(i)) continue;
        for (let attempt = 0; ; attempt++) {
          try {
            await api.putChunk(id, i, f.slice(i * chunk, Math.min(f.size, (i + 1) * chunk)), (l) => setPct(Math.min(99, Math.floor(((done + l) / f.size) * 100))), ctl.signal);
            break;
          } catch (e) {
            if ((e as Error).name === "AbortError") { setPhase("paused"); return; }
            const retryable = !(e instanceof ApiError) || e.status === 0 || e.status >= 500;
            if (!retryable || attempt >= 3) throw e;
            await sleep(800 * (attempt + 1));
          }
        }
        done += Math.min(chunk, f.size - i * chunk);
        setPct(Math.min(99, Math.floor((done / f.size) * 100)));
      }
      setPct(100);
      const r = await api.uploadComplete(id);
      await pollJob(r.job_id);
    } catch (e) {
      if ((e as Error).name === "AbortError") { setPhase("paused"); return; }
      fail(e, m.uploadFailedTitle);
    }
  }, [fail, pollJob]);

  const begin = useCallback(async (f: File) => {
    setErr(null); setFile(f); setPct(0);
    if (f.size > limits.max_upload_bytes) {
      setErr({ message: m.fileTooLarge(formatBytes(f.size), formatBytes(limits.max_upload_bytes)), next: m.fileTooLargeNext, code: "limit_exceeded" });
      setPhase("failed"); return;
    }
    try {
      let ws = workspace;
      if (!ws) { ws = await api.createWorkspace(f.name.replace(/[\u0000-\u001f\u007f]/g, "").slice(0, 100) || "archive"); onWorkspace(ws); }
      wsId.current = ws.id;
      const saved = loadSaved();
      if (saved && saved.name === f.name && saved.size === f.size && saved.lastModified === f.lastModified && saved.workspaceId === ws.id) {
        try { const s = await api.uploadStatus(saved.uploadId); if (!s.completed) { uploadId.current = saved.uploadId; await sendAll(f, saved.uploadId); return; } } catch { /* start fresh */ }
      }
      let sha: string | undefined;
      if (verify && f.size <= MAX_HASH_BYTES) { setPhase("hashing"); sha = await sha256Hex(f); }
      const s = await api.uploadStart(ws.id, f.name, f.size, sha);
      uploadId.current = s.upload_id;
      store({ uploadId: s.upload_id, name: f.name, size: f.size, lastModified: f.lastModified, workspaceId: ws.id });
      await sendAll(f, s.upload_id);
    } catch (e) { fail(e, m.uploadFailedTitle); }
  }, [limits, workspace, verify, onWorkspace, sendAll, fail]);

  const cancel = async () => {
    abort.current?.abort();
    const id = uploadId.current; uploadId.current = null; store(null);
    if (id) { try { await api.uploadAbort(id); } catch { /* already gone */ } }
    setPhase("idle"); setFile(null); setErr(null);
  };

  const busy = phase === "hashing" || phase === "uploading" || phase === "processing";
  const input = useRef<HTMLInputElement>(null);
  return (
    <section aria-labelledby="upload-h" className="panel">
      <h2 id="upload-h">{m.uploadHeading}</h2>
      <p>{m.uploadHelp(formatBytes(limits.max_upload_bytes))}</p>
      <div
        className="drop"
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => { e.preventDefault(); const f = e.dataTransfer.files[0]; if (f && !busy) void begin(f); }}
      >
        <input ref={input} id="file" type="file" aria-label={m.fileInputLabel} className="visually-hidden" disabled={busy}
          onChange={(e) => { const f = e.target.files?.[0]; if (f) void begin(f); e.target.value = ""; }} />
        <button type="button" disabled={busy} onClick={() => input.current?.click()}>{m.chooseFile}</button>
        <span className="muted">{m.dropHere}</span>
      </div>
      <label className="check"><input type="checkbox" checked={verify} disabled={busy} onChange={(e) => setVerify(e.target.checked)} /> {m.verifyHash}</label>
      <div aria-live="polite" className="status">
        {phase === "hashing" && <p>{m.hashing}</p>}
        {(phase === "uploading" || phase === "paused") && file && (
          <div>
            <p>{file.name}: {phase === "paused" ? m.paused : m.uploading(pct)}</p>
            <progress aria-label={m.uploadProgressLabel} max={100} value={pct} />
            <div className="row">
              {phase === "uploading" ? <button onClick={() => abort.current?.abort()}>{m.pause}</button> : <button onClick={() => file && uploadId.current && void sendAll(file, uploadId.current)}>{m.resume}</button>}
              <button className="secondary" onClick={() => void cancel()}>{m.cancelUpload}</button>
            </div>
          </div>
        )}
        {phase === "processing" && (<div><p>{m.processing}</p><progress aria-label={m.processingLabel} max={100} value={jobPct} /></div>)}
      </div>
      {phase === "failed" && err && (
        <ErrorBox err={err}
          onRetry={file && uploadId.current ? () => void sendAll(file, uploadId.current!) : undefined}
          retryLabel={m.resume} />
      )}
      {phase === "failed" && err && <button className="secondary" onClick={() => void cancel()}>{m.cancel}</button>}
      {phase === "failed" && err?.code !== "limit_exceeded" && uploadId.current && <p className="muted">{m.resumeHint}</p>}
    </section>
  );
}
