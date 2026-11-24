import { useCallback, useEffect, useState } from "react";
import * as api from "./api";
import { Limits, Me, VersionInfo, Workspace } from "./api";
import { Browser } from "./Browser";
import { ErrorBox, Shown, toShown } from "./ErrorBox";
import { formatBytes } from "./format";
import { m } from "./messages";
import { UploadPanel } from "./UploadPanel";

export function App() {
  const [boot, setBoot] = useState<"loading" | "ready" | "lost" | "error">("loading");
  const [err, setErr] = useState<Shown | null>(null);
  const [limits, setLimits] = useState<Limits | null>(null);
  const [me, setMe] = useState<Me | null>(null);
  const [ws, setWs] = useState<Workspace | null>(null);
  const [versions, setVersions] = useState<VersionInfo[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [showUpload, setShowUpload] = useState(true);
  const [confirm, setConfirm] = useState(false);
  const [theme, setTheme] = useState<string>(() => { try { return localStorage.getItem("theme") || "auto"; } catch { return "auto"; } });
  useEffect(() => {
    if (theme === "auto") document.documentElement.removeAttribute("data-theme"); else document.documentElement.setAttribute("data-theme", theme);
    try { localStorage.setItem("theme", theme); } catch { /* storage may be unavailable */ }
  }, [theme]);

  const refresh = useCallback(async (select?: string) => {
    const [l, u, list] = await Promise.all([api.getLimits(), api.getMe(), api.listWorkspaces()]);
    setLimits(l); setMe(u);
    const w = list[0] || null; setWs(w);
    const vs = w ? await api.listVersions(w.id) : [];
    setVersions(vs);
    const ready = vs.filter((v) => v.state === "ready");
    setActive((cur) => select || (cur && ready.some((v) => v.id === cur) ? cur : ready.length ? ready[ready.length - 1].id : null));
    setShowUpload(ready.length === 0);
  }, []);

  const start = useCallback(async (forceNew = false) => {
    setBoot("loading"); setErr(null);
    try { if (!(forceNew ? false : await api.resumeSession())) await api.startSession(); await refresh(); setBoot("ready"); }
    catch (e) { setErr(toShown(e)); setBoot("error"); }
  }, [refresh]);
  useEffect(() => { void start(); }, [start]);

  const lost = useCallback(() => setBoot("lost"), []);
  const guard = async (f: () => Promise<void>) => {
    try { await f(); } catch (e) { if (e instanceof api.ApiError && e.code === "unauthenticated") setBoot("lost"); else setErr(toShown(e)); }
  };

  if (boot === "loading") return <main id="main" className="center"><p role="status">{m.loading}</p></main>;
  if (boot === "error") return <main id="main" className="center">{err && <ErrorBox err={err} onRetry={() => void start()} />}</main>;
  if (boot === "lost") return (
    <main id="main" className="center"><div className="panel"><h1>{m.sessionExpiredTitle}</h1><button onClick={() => void start(true)}>{m.startNewSession}</button></div></main>
  );
  const ready = versions.filter((v) => v.state === "ready");
  return (
    <>
      <a className="skip" href="#main">{m.skipToContent}</a>
      <header className="top">
        <div><h1>{m.appName}</h1><p className="muted">{m.tagline}</p></div>
        <div className="usage">
          <label>{m.themeLabel}{" "}
            <select value={theme} onChange={(e) => setTheme(e.target.value)}>
              <option value="auto">{m.themeAuto}</option><option value="light">{m.themeLight}</option><option value="dark">{m.themeDark}</option>
            </select>
          </label>
          {me && <p>{m.storageUsed(formatBytes(me.usage_bytes), formatBytes(me.quota_bytes))}</p>}
          {limits && <p className="muted">{m.retentionNotice(limits.idle_hours)}</p>}
          {!confirm ? <button className="secondary" onClick={() => setConfirm(true)}>{m.deleteMyData}</button> : (
            <div role="alertdialog" aria-label={m.deleteMyData} className="row">
              <span>{m.deleteMyDataConfirm}</span>
              <button className="danger" onClick={() => void guard(async () => { await api.deleteMe(); setConfirm(false); await start(true); })}>{m.deleteConfirmYes}</button>
              <button className="secondary" onClick={() => setConfirm(false)}>{m.cancel}</button>
            </div>
          )}
        </div>
      </header>
      <main id="main">
        {err && <ErrorBox err={err} />}
        {ws && ready.length > 0 && (
          <section className="panel" aria-labelledby="ws-h">
            <h2 id="ws-h">{m.workspaceHeading}</h2>
            <div className="row">
              <label>{m.versionLabel}{" "}
                <select value={active || ""} onChange={(e) => setActive(e.target.value)}>
                  {ready.map((v) => <option key={v.id} value={v.id}>{m.versionOption(v.filename, v.entry_count)}</option>)}
                </select>
              </label>
              <button className="secondary" onClick={() => setShowUpload((s) => !s)} aria-expanded={showUpload}>{m.uploadAnother}</button>
              <button className="secondary" onClick={() => void guard(async () => { await api.deleteWorkspace(ws.id); await refresh(); })}>{m.deleteWorkspace}</button>
            </div>
          </section>
        )}
        {limits && showUpload && (
          <UploadPanel limits={limits} workspace={ws} onWorkspace={setWs} onSessionLost={lost}
            onReady={(v) => void guard(async () => { await refresh(v); setShowUpload(false); })} />
        )}
        {!ws && !showUpload && <p>{m.noArchives}</p>}
        {active && limits && <Browser versionId={active} workspaceId={ws ? ws.id : null} versions={versions} limits={limits} onSaved={(v) => void guard(async () => { await refresh(v); })} onChanged={() => void guard(async () => { await refresh(); })} />}
      </main>
    </>
  );
}
