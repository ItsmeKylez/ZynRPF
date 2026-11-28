// Web shell (M3). Intentionally thin: workspace file list, upload, document tabs, and the existing <Belge> view.
// Desktop-only behaviour (mods folder, native drag-drop positions, Explorer integration) is not carried over.
import { useCallback, useEffect, useRef, useState } from "react";
import { api, boyutYaz, type BelgeOzet } from "../api";
import { Belge } from "../Belge";
import { Backups } from "./Backups";
import { STALE_EVENT, refreshDocument } from "./transport";
import { Ikon, useArayuz } from "../ortak";
import { fileIdFromHandle, webFileHandle } from "./adapt";
import { http } from "./http";
import { UNAVAILABLE_EVENT, notifyUnavailable } from "./unavailable";
import type { WireFile, WireSession } from "./wire";

const hours = (s: number) => (s % 3600 === 0 ? `${s / 3600} h` : `${Math.round(s / 60)} min`);

export default function WebApp() {
  const ui = useArayuz();
  const [files, setFiles] = useState<WireFile[]>([]);
  const [session, setSession] = useState<WireSession | null>(null);
  const [belgeler, setBelgeler] = useState<BelgeOzet[]>([]);
  const [aktif, setAktif] = useState<number | null>(null);
  const [surumler, setSurumler] = useState<Record<number, number>>({});
  const [secimler, setSecimler] = useState<Record<number, string>>({});
  const [surukleniyor, setSurukleniyor] = useState(false);
  const [yedekAcik, setYedekAcik] = useState(false);
  const belgelerRef = useRef<BelgeOzet[]>([]);
  const input = useRef<HTMLInputElement>(null);
  // `ui` changes identity whenever the provider re-renders (every toast). Effects that must not re-run use this ref.
  const uiRef = useRef(ui);
  uiRef.current = ui;

  belgelerRef.current = belgeler;

  const refreshFiles = useCallback(async () => {
    try {
      setFiles(await http.listFiles());
    } catch (e) {
      ui.hata(e);
    }
  }, [ui]);

  const guncelle = useCallback((o: BelgeOzet) => {
    setBelgeler((b) => (b.some((x) => x.id === o.id) ? b.map((x) => (x.id === o.id ? o : x)) : [...b, o]));
    setSurumler((s) => ({ ...s, [o.id]: (s[o.id] ?? 0) + 1 }));
    // An edit (rename, move, delete, undo, restore...) can remove the selected entry. Clear such a selection in the same
    // render, otherwise the viewer asks the server for a path that no longer exists.
    const paths = new Set<string>();
    const walk = (ns: BelgeOzet["agac"]) => ns.forEach((n) => (paths.add(n.yol), walk(n.cocuklar)));
    walk(o.agac);
    setSecimler((s) => {
      const sel = s[o.id];
      if (!sel || sel.startsWith("hazir:") || sel === "harita-editoru" || paths.has(sel.split("#")[0])) return s;
      return { ...s, [o.id]: "" };
    });
  }, []);

  // First load only (never keyed on `ui`, or a failing request would re-trigger itself through its own error toast).
  // The session cookie keeps the workspace across reloads, so restore files and open documents.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // Sequential on purpose: the first request creates the workspace and sets the cookie. Firing the three
        // requests in parallel with no cookie would make the server create three workspaces for one page load.
        const s = await http.session();
        const [f, open] = await Promise.all([http.listFiles(), api.acikBelgeler()]);
        if (cancelled) return;
        setSession(s);
        setFiles(f);
        const ozetler = await Promise.all(open.map((d) => api.ozet(d.id)));
        if (cancelled) return;
        ozetler.forEach(guncelle);
        if (ozetler.length) setAktif(ozetler[0].id);
      } catch (e) {
        if (!cancelled) uiRef.current.hata(e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [guncelle]);

  useEffect(() => {
    const on = (e: Event) => uiRef.current.bildir((e as CustomEvent<string>).detail, "hata");
    addEventListener(UNAVAILABLE_EVENT, on);
    return () => removeEventListener(UNAVAILABLE_EVENT, on);
  }, []);

  const openFile = useCallback(
    async (f: WireFile) => {
      const mevcut = belgeler.find((b) => b.yol === webFileHandle(f.fileId));
      if (mevcut) return setAktif(mevcut.id);
      const o = await ui.mesgul(`Opening ${f.name}`, () => api.arsivAc(webFileHandle(f.fileId)));
      if (o) {
        guncelle(o);
        setAktif(o.id);
      }
    },
    [belgeler, guncelle, ui],
  );

  const upload = useCallback(
    async (list: FileList | File[]) => {
      for (const file of Array.from(list)) {
        if (!/\.rpf$/i.test(file.name)) {
          ui.hata(new Error(`"${file.name}" is not an .rpf file.`));
          continue;
        }
        const meta = await ui.mesgul(`Uploading ${file.name}`, () => http.upload(file));
        if (meta) {
          await refreshFiles();
          await openFile(meta);
        }
      }
    },
    [openFile, refreshFiles, ui],
  );

  const kapat = useCallback(
    async (id: number) => {
      try {
        const b = belgelerRef.current.find((x) => x.id === id);
        if (b?.kirli && !(await ui.onayla("Unsaved changes", `Close ${b.ad} without saving? Your changes will be lost.`, "Close without saving", true))) return;
        await api.kapat(id);
        setBelgeler((b) => b.filter((x) => x.id !== id));
        setAktif((a) => (a === id ? (belgeler.find((x) => x.id !== id)?.id ?? null) : a));
      } catch (e) {
        ui.hata(e);
      }
    },
    [belgeler, ui],
  );

  // "Save" in the web version = download the verified re-serialized archive (the server never overwrites the upload).
  const indir = (id: number) => {
    const a = document.createElement("a");
    a.href = http.exportUrl(id);
    a.download = belgeler.find((b) => b.id === id)?.ad ?? "archive.rpf";
    document.body.appendChild(a);
    a.click();
    a.remove();
    ui.bildir("Downloading the archive as it is now. Your uploaded original is unchanged.", "bilgi");
  };

  // Save = serialize, re-parse to verify, keep the previous saved version as a backup, atomically replace the workspace copy.
  // The uploaded original is never modified. "Download archive" gets the current state (including unsaved edits) as a file.
  const kaydet = useCallback(
    async (id: number) => {
      try {
        const r = await ui.mesgul("Saving", () => api.kaydet(id));
        if (!r) return;
        guncelle(r.ozet);
        const sizes = r.eski_boyut > 0 ? `${boyutYaz(r.eski_boyut)} → ${boyutYaz(r.yeni_boyut)}` : boyutYaz(r.yeni_boyut);
        ui.bildir(`Saved to your workspace · ${sizes}${r.yedek ? " · previous version backed up" : ""}`, "tamam");
      } catch (e) {
        ui.hata(e);
      }
    },
    [guncelle, ui],
  );

  const geriIleri = useCallback(
    async (id: number, yon: "geri" | "ileri") => {
      try {
        guncelle(await (yon === "geri" ? api.geriAl(id) : api.yinele(id)));
      } catch (e) {
        ui.hata(e);
      }
    },
    [guncelle, ui],
  );

  // A stale-edit refusal (412) means another tab changed the document: reload its state so the next try works.
  useEffect(() => {
    const on = (e: Event) => {
      const id = (e as CustomEvent<number>).detail;
      refreshDocument(id).then((o) => guncelle(o), () => {});
    };
    addEventListener(STALE_EVENT, on);
    return () => removeEventListener(STALE_EVENT, on);
  }, [guncelle]);

  // Leaving the page loses unsaved in-memory edits (saved ones stay in the workspace until it expires).
  useEffect(() => {
    const on = (e: BeforeUnloadEvent) => {
      if (belgelerRef.current.some((b) => b.kirli)) e.preventDefault();
    };
    addEventListener("beforeunload", on);
    return () => removeEventListener("beforeunload", on);
  }, []);

  const aktifBelge = belgeler.find((b) => b.id === aktif);
  const aktifDosya = aktifBelge?.yol ? fileIdFromHandle(aktifBelge.yol) : null;

  return (
    <div
      className="uygulama"
      onDragOver={(e) => {
        if (e.dataTransfer.types.includes("Files")) {
          e.preventDefault();
          setSurukleniyor(true);
        }
      }}
      onDragLeave={(e) => e.currentTarget === e.target && setSurukleniyor(false)}
      onDrop={(e) => {
        e.preventDefault();
        setSurukleniyor(false);
        if (e.dataTransfer.files.length) upload(e.dataTransfer.files);
      }}
    >
      <input
        ref={input}
        type="file"
        accept=".rpf"
        multiple
        hidden
        onChange={(e) => {
          if (e.target.files) upload(e.target.files);
          e.target.value = "";
        }}
      />
      <aside className="yan">
        <div className="marka">
          <img src="/ikon.svg" alt="" />
          <div>
            <b>tlk-rpf</b>
            <br />
            <span>Web workspace</span>
          </div>
        </div>
        <div className="yan-eylemler">
          <button className="dugme birincil" onClick={() => input.current?.click()}>
            <Ikon ad="klasor-ac" /> Upload RPF
          </button>
        </div>
        <div className="bolum-baslik">
          <span>Workspace files</span>
          <span>
            <button className="ikon-dugme" title="Refresh" onClick={refreshFiles}>
              <Ikon ad="yenile" />
            </button>
          </span>
        </div>
        <div className="mod-listesi">
          {files.length === 0 && <div className="bilgi-kutu" style={{ margin: 8 }}>No files yet. Upload an .rpf to begin.</div>}
          {files.map((f) => {
            const acik = belgeler.find((b) => b.yol === webFileHandle(f.fileId));
            return (
              <div key={f.fileId} className={`mod${acik && acik.id === aktif ? " secili" : ""}`} onClick={() => openFile(f)} title={f.name}>
                <div className="mod-ad">{f.name}</div>
                <div className="mod-alt">
                  <span>{boyutYaz(f.size)}</span>
                  {acik && <span className="rozet">open</span>}
                </div>
              </div>
            );
          })}
        </div>
        {session && (
          <div className="bilgi-kutu" style={{ margin: 8 }}>
            <small>
              Your workspace is temporary: files are deleted after {hours(session.limits.idleTtlSeconds)} of inactivity (at most{" "}
              {hours(session.limits.maxAgeSeconds)}). Download anything you want to keep. Upload limit {boyutYaz(session.limits.maxUploadBytes)}.
            </small>
          </div>
        )}
      </aside>

      <main className="ana">
        {belgeler.length > 0 && (
          <div className="sekmeler">
            {belgeler.map((b) => (
              <div key={b.id} className={`sekme${b.id === aktif ? " secili" : ""}`} onClick={() => setAktif(b.id)} title={b.ad}>
                <Ikon ad="arsiv" />
                <span>{b.ad}</span>
                <button
                  className="ikon-dugme"
                  style={{ width: 20, height: 20 }}
                  title="Close"
                  onClick={(e) => {
                    e.stopPropagation();
                    kapat(b.id);
                  }}
                >
                  <Ikon ad="x" boy={12} />
                </button>
              </div>
            ))}
            {aktifBelge && (
              <span style={{ marginLeft: "auto", alignSelf: "center", marginRight: 8, display: "flex", gap: 6 }}>
                {aktifDosya && (
                  <button className="dugme kucuk" onClick={() => setYedekAcik(true)}>
                    <Ikon ad="yedek" /> Backups
                  </button>
                )}
                <button className="dugme kucuk" onClick={() => indir(aktifBelge.id)} title="Downloads the archive as it is now, including unsaved changes">
                  Download archive
                </button>
              </span>
            )}
          </div>
        )}
        {aktifBelge ? (
          <Belge
            key={aktifBelge.id}
            ozet={aktifBelge}
            surum={surumler[aktifBelge.id] ?? 0}
            secim={secimler[aktifBelge.id] ?? ""}
            secimAyarla={(y) => setSecimler((s) => ({ ...s, [aktifBelge.id]: y }))}
            guncelle={guncelle}
            kaydet={(farkli) => (farkli ? indir(aktifBelge.id) : kaydet(aktifBelge.id))}
            geriIleri={(yon) => geriIleri(aktifBelge.id, yon)}
            birakmaHedefi={null}
            gezgindeGoster={() => notifyUnavailable("Showing a file in its folder")}
            modsKlasoru={undefined}
            modlarDegisti={() => {}}
          />
        ) : (
          <div className={`bos-ekran${surukleniyor ? " hedef" : ""}`}>
            <div className="kutu">
              <Ikon ad="arsiv" boy={40} renk="var(--vurgu)" />
              <h2>Drop an .rpf here or upload one</h2>
              <div>Your archive is uploaded to a private, temporary workspace. You can browse, edit, import, export and save it. Saving keeps the original upload and backs up the previous version.</div>
              <div style={{ marginTop: 16, display: "flex", gap: 8, justifyContent: "center" }}>
                <button className="dugme birincil" onClick={() => input.current?.click()}>
                  <Ikon ad="klasor-ac" /> Upload RPF
                </button>
              </div>
            </div>
          </div>
        )}
      </main>
      {yedekAcik && aktifBelge && aktifDosya && <Backups belge={aktifBelge} fileId={aktifDosya} kapat={() => setYedekAcik(false)} guncelle={guncelle} />}
    </div>
  );
}
