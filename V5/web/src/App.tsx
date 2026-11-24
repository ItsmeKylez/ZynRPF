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
  const [closed, setClosed] = useState<Set<string>>(new Set());
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
    if (select) setClosed((c) => { if (!c.has(select)) return c; const n = new Set(c); n.delete(select); return n; });
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
  const tabs = ready.filter((v) => !closed.has(v.id));
  const closeTab = (id: string) => {
    const i = tabs.findIndex((v) => v.id === id);
    setClosed((c) => new Set(c).add(id));
    if (active === id) { const rest = tabs.filter((v) => v.id !== id); setActive(rest.length ? rest[Math.min(i, rest.length - 1)].id : null); }
  };
  const openTab = (id: string) => { setClosed((c) => { const n = new Set(c); n.delete(id); return n; }); setActive(id); };
  const onTabKey = (ev: React.KeyboardEvent, i: number) => {
    const d = ev.key === "ArrowRight" ? 1 : ev.key === "ArrowLeft" ? -1 : 0;
    if (!d || tabs.length === 0) return;
    ev.preventDefault();
    const t = tabs[(i + d + tabs.length) % tabs.length]; setActive(t.id);
    requestAnimationFrame(() => document.getElementById("tab-" + t.id)?.focus());
  };
  const recent = [...ready].sort((a, b) => (b.created_at || 0) - (a.created_at || 0));
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
            <div className="tabs" role="tablist" aria-label={m.tabsLabel}>
              {tabs.map((v, i) => (
                <div key={v.id} role="presentation" className={"tab" + (v.id === active ? " active" : "")}>
                  <button role="tab" id={"tab-" + v.id} aria-selected={v.id === active} tabIndex={v.id === active ? 0 : -1} onClick={() => setActive(v.id)} onKeyDown={(e) => onTabKey(e, i)} title={v.filename}>
                    {v.origin === "nested" ? `${v.filename} (${m.tabNested})` : v.filename}
                  </button>
                  <button className="tab-close" aria-label={m.closeTab(v.filename)} onClick={() => closeTab(v.id)}>{m.closeTabShort}</button>
                </div>
              ))}
            </div>
            <div className="row">
              <button className="secondary" onClick={() => setShowUpload((s) => !s)} aria-expanded={showUpload}>{m.uploadAnother}</button>
              <button className="secondary" onClick={() => void guard(async () => { await api.deleteWorkspace(ws.id); await refresh(); })}>{m.deleteWorkspace}</button>
            </div>
          </section>
        )}
        {ready.length > 0 && (
          <details className="panel recent">
            <summary>{m.recentHeading(recent.length)}</summary>
            <p className="muted">{m.recentNote}</p>
            <ul aria-label={m.recentHeading(recent.length)}>
              {recent.map((v) => (
                <li key={v.id}>
                  <strong>{v.filename}</strong> <span className="muted">{m.recentMeta(v.entry_count, v.created_at ? new Date(v.created_at * 1000).toLocaleString() : "")}</span>{" "}
                  {closed.has(v.id) || v.id !== active ? <button className="secondary" onClick={() => openTab(v.id)}>{closed.has(v.id) ? m.recentOpen : m.recentOpenNow}</button> : null}
                </li>
              ))}
            </ul>
          </details>
        )}
        {limits && showUpload && (
          <UploadPanel limits={limits} workspace={ws} onWorkspace={setWs} onSessionLost={lost}
            onReady={(v) => void guard(async () => { await refresh(v); setShowUpload(false); })} />
        )}
        {!ws && !showUpload && <p>{m.noArchives}</p>}
        {active && !closed.has(active) && limits && <Browser versionId={active} workspaceId={ws ? ws.id : null} versions={versions} limits={limits} onSaved={(v) => void guard(async () => { await refresh(v); })} onChanged={() => void guard(async () => { await refresh(); })} onOpenNested={(v) => void guard(async () => { await refresh(v); })} />}
      </main>
    </>
  );
}
