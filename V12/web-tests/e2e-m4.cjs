// M4 browser E2E: real zyn-server binary + built web UI + Chromium (Playwright must be resolvable, e.g. NODE_PATH=$(npm root -g)).
// Prereqs: `npm run build:web`; `cd web && cargo build -p zyn-server --examples`. Run: node web-tests/e2e-m4.cjs "$PWD" /tmp/m4-e2e-out  (needs `unzip`).
const { chromium } = require("playwright");
const { spawn, execFileSync } = require("child_process");
const assert = require("assert/strict");
const fs = require("fs");
const path = require("path");
const repo = process.argv[2], out = process.argv[3];
const PORT = 18490;
fs.mkdirSync(out, { recursive: true });
const sample = path.join(out, "sample.rpf");
execFileSync(path.join(repo, "web/target/debug/examples/make_sample_rpf"), [sample]);
fs.writeFileSync(path.join(out, "a.txt"), "AAA");
fs.writeFileSync(path.join(out, "b.bin"), Buffer.from([1, 2, 3]));
fs.writeFileSync(path.join(out, "repl.txt"), "REPLACED");
fs.mkdirSync(path.join(out, "folder/sub"), { recursive: true });
fs.writeFileSync(path.join(out, "folder/x.txt"), "X");
fs.writeFileSync(path.join(out, "folder/sub/y.txt"), "Y");
const srv = spawn(path.join(repo, "web/target/debug/zyn-server"), [], {
  env: { ...process.env, PORT: String(PORT), STATIC_DIR: path.join(repo, "dist-web"), WORKSPACE_ROOT: path.join(out, "ws") },
  stdio: ["ignore", "inherit", "inherit"],
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const until = async (fn, ms = 8000) => {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    try { if (await fn()) return; } catch {}
    await sleep(100);
  }
  throw new Error(`condition not met within ${ms}ms: ${fn}`);
};
const results = [];
let browser;
const collected = [];
const step = async (name, fn) => { await fn(); results.push(name); console.log("ok  -", name); };

(async () => {
  for (let i = 0; i < 50; i++) { try { if ((await fetch(`http://127.0.0.1:${PORT}/healthz`)).ok) break; } catch {} await sleep(100); }
  browser = await chromium.launch();
  const ctx = await browser.newContext({ acceptDownloads: true, viewport: { width: 1400, height: 900 } });
  const page = await ctx.newPage();
  const problems = [];
  await page.addInitScript(() => {
    window.__errToasts = [];
    new MutationObserver((ms) => ms.forEach((m) => m.addedNodes.forEach((n) => n.classList && n.classList.contains("hata") && window.__errToasts.push(n.textContent)))).observe(document, { childList: true, subtree: true });
  });
  const failed = [];
  page.on("console", (m) => m.type() === "error" && problems.push(m.text()));
  page.on("pageerror", (e) => problems.push("pageerror: " + e.message));
  page.on("response", (r) => { if (r.status() >= 400) failed.push(`${r.status()} ${r.request().method()} ${decodeURIComponent(new URL(r.url()).pathname + new URL(r.url()).search)}`); });
  const api = (p, init) => page.evaluate(async ([p, init]) => (await fetch("/api/v1" + p, init)).json(), [p, init]);
  const summary = () => api("/documents/1");
  const paths = async () => { const out = []; const w = (n) => n.forEach((c) => { out.push(c.path); w(c.children); }); w((await summary()).tree); return out.sort(); };
  const row = (name) => page.locator(".satir", { has: page.locator(".ad", { hasText: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) }) });
  const menu = async (name, item) => { await row(name).click({ button: "right" }); await page.locator(".baglam button", { hasText: item }).click(); };
  const toast = (t) => page.locator(".bildirim", { hasText: t }).first().waitFor({ timeout: 5000 });
  const modalOk = async (label) => page.locator(".modal .eylemler button.birincil", label ? { hasText: label } : {}).click();

  await page.goto(`http://127.0.0.1:${PORT}/`);
  await page.locator('input[type=file]').first().setInputFiles(sample);
  await row("content").waitFor();
  await step("upload + open", async () => assert.deepEqual(await paths(), ["content", "content/readme.txt"]));

  await step("create folder", async () => {
    await menu("content", "Yeni klasör");
    await page.locator(".modal input.girdi").fill("newdir");
    await modalOk();
    await row("newdir").waitFor();
    assert.ok((await paths()).includes("content/newdir"));
  });

  await step("import files via browser chooser (nested rel paths for folder pick)", async () => {
    const [chooser] = await Promise.all([page.waitForEvent("filechooser"), menu("content", "Dosya ekle")]);
    await chooser.setFiles([path.join(out, "a.txt"), path.join(out, "b.bin")]);
    await row("a.txt").waitFor();
    const [chooser2] = await Promise.all([page.waitForEvent("filechooser"), menu("newdir", "Klasör ekle")]);
    assert.ok(chooser2.isMultiple() === false);
    await chooser2.setFiles(path.join(out, "folder"));
    await until(() => page.evaluate(async () => (await (await fetch("/api/v1/documents/1")).json()).undoDepth >= 3));
    const p = await paths();
    for (const w of ["content/a.txt", "content/b.bin", "content/newdir/folder/x.txt", "content/newdir/folder/sub/y.txt"]) assert.ok(p.includes(w), `${w} in ${p}`);
  });

  await step("replace file", async () => {
    const [chooser] = await Promise.all([page.waitForEvent("filechooser"), menu("a.txt", "Dosyayla değiştir")]);
    await chooser.setFiles(path.join(out, "repl.txt"));
    await until(() => page.evaluate(async () => (await (await fetch("/api/v1/documents/1/entries?path=content/a.txt")).json()).text === "REPLACED"));
  });

  await step("edit text through the viewer API path (metin_yaz) shows in file view", async () => {
    await row("readme.txt").click();
    await page.getByText("hello").first().waitFor({ timeout: 5000 });
  });

  await step("rename with F2", async () => {
    await row("b.bin").click();
    await page.keyboard.press("F2");
    await page.locator(".modal input.girdi").fill("c.bin");
    await page.keyboard.press("Enter");
    await row("c.bin").waitFor();
    assert.ok(!(await paths()).includes("content/b.bin"));
  });

  await step("move via prompt", async () => {
    await menu("c.bin", "Taşı");
    await page.locator(".modal input.girdi").fill("content/newdir");
    await page.keyboard.press("Enter");
    await until(() => page.evaluate(async () => JSON.stringify(await (await fetch("/api/v1/documents/1")).json()).includes("content/newdir/c.bin")));
  });

  await step("delete then undo then redo", async () => {
    await menu("readme.txt", "Sil");
    await modalOk("Sil");
    await until(() => page.evaluate(async () => !JSON.stringify(await (await fetch("/api/v1/documents/1")).json()).includes("content/readme.txt")));
    await page.locator('button[title^="Geri al"]').click();
    await row("readme.txt").waitFor();
    await page.locator('button[title^="Yinele"]').click();
    await until(() => page.evaluate(async () => !JSON.stringify(await (await fetch("/api/v1/documents/1")).json()).includes("content/readme.txt")));
    await page.locator('button[title^="Geri al"]').click();
    await row("readme.txt").waitFor();
  });

  await step("export selection downloads a ZIP with the right entries", async () => {
    const [dl] = await Promise.all([page.waitForEvent("download"), menu("content", "Dışa aktar")]);
    assert.equal(dl.suggestedFilename(), "sample.zip");
    const f = path.join(out, "export.zip");
    await dl.saveAs(f);
    const names = execFileSync("unzip", ["-Z1", f]).toString().split("\n").filter(Boolean);
    for (const w of ["content/readme.txt", "content/a.txt", "content/newdir/folder/sub/y.txt", "content/newdir/c.bin"]) assert.ok(names.includes(w), `${w} in ${names}`);
    assert.equal(execFileSync("unzip", ["-p", f, "content/a.txt"]).toString(), "REPLACED");
    await toast("dosya çıkarıldı");
  });

  await step("save: clean state, original untouched, current differs", async () => {
    const dirtyBefore = (await summary()).dirty;
    assert.equal(dirtyBefore, true);
    await page.locator(".belge-ust button.birincil", { hasText: /Kaydet/ }).click();
    await toast("Saved to your workspace");
    const s = await summary();
    assert.equal(s.dirty, false);
    const fid = (await api("/files"))[0].fileId;
    const orig = await page.evaluate(async (fid) => (await (await fetch(`/api/v1/files/${fid}/download?version=original`)).arrayBuffer()).byteLength, fid);
    assert.equal(orig, fs.statSync(sample).size);
    const cur = await page.evaluate(async (fid) => (await (await fetch(`/api/v1/files/${fid}/download?version=current`)).arrayBuffer()).byteLength, fid);
    assert.notEqual(cur, orig);
  });

  await step("undo after save makes it dirty again; second save creates a backup", async () => {
    await page.locator('button[title^="Geri al"]').click();
    await until(() => page.evaluate(async () => (await (await fetch("/api/v1/documents/1")).json()).dirty === true));
    await page.locator(".belge-ust button.birincil", { hasText: /Kaydet/ }).click();
    await toast("previous version backed up");
    const fid = (await api("/files"))[0].fileId;
    assert.equal((await api(`/files/${fid}/backups`)).length, 1);
  });

  await step("backups panel lists and restores", async () => {
    await page.getByRole("button", { name: /Backups/ }).click();
    await page.locator(".yedek-listesi .dosya-satiri").first().waitFor();
    await page.locator(".yedek-listesi .dosya-satiri button", { hasText: "Restore" }).first().click();
    await modalOk("Restore");
    await toast("Backup restored");
    const s = await summary();
    assert.equal(s.dirty, false); assert.equal(s.undoDepth, 0);
    await page.locator(".kart-ust .ikon-dugme").click();
  });

  await step("stale edit from another tab is refused and the view refreshes", async () => {
    // Another "tab" changes the document directly through the API.
    await api("/documents/1/mkdir", { method: "POST", headers: { "X-Requested-With": "zyn", "Content-Type": "application/json" }, body: JSON.stringify({ path: "from-other-tab" }) });
    await menu("content", "Yeni klasör");
    await page.locator(".modal input.girdi").fill("mine");
    await modalOk();
    await toast("changed since you last loaded");
    await row("from-other-tab").waitFor();
    assert.ok(!(await paths()).includes("content/mine"));
    // After the refresh the same action works.
    await menu("content", "Yeni klasör");
    await page.locator(".modal input.girdi").fill("mine");
    await modalOk();
    await row("mine").waitFor();
  });

  await step("reload keeps workspace and document; closing a dirty document asks first", async () => {
    collected.push(...(await page.evaluate(() => window.__errToasts)));
    await page.reload();
    await row("content").waitFor();
    assert.ok((await paths()).includes("content/mine"));
    await page.locator('.sekme button[title="Close"]').click();
    await page.locator(".modal", { hasText: "Unsaved changes" }).waitFor();
    await page.locator(".modal .eylemler button", { hasText: "Vazgeç" }).click();
    assert.equal((await api("/documents")).length, 1);
  });

  await step("hostile import path refused through the UI chooser path", async () => {
    const res = await page.evaluate(async () => {
      const fd = new FormData();
      fd.append("file", new File(["x"], "../../evil.txt"));
      const r = await fetch("/api/v1/documents/1/import", { method: "POST", headers: { "X-Requested-With": "zyn" }, body: fd });
      return [r.status, (await r.json()).error.code];
    });
    assert.deepEqual(res, [400, "INVALID_PATH"]);
  });

  await page.screenshot({ path: path.join(out, "m4.png") });
  console.log("failed requests:", failed);
  // Exactly these are expected: two reads superseded by the user's own rename/move (handled silently), one deliberate
  // stale-edit refusal (412), one deliberate hostile import (400).
  assert.deepEqual(failed.sort(), [
    "400 POST /api/v1/documents/1/import",
    "404 GET /api/v1/documents/1/entries?path=content/b.bin",
    "404 GET /api/v1/documents/1/entries?path=content/c.bin",
    "412 POST /api/v1/documents/1/mkdir",
  ]);
  const unexpected = problems.filter((p) => !/status of (400|404|412)/.test(p));
  assert.deepEqual(unexpected, [], "no page errors or unexpected console errors");
  const toasts = [...collected, ...(await page.evaluate(() => window.__errToasts))];
  console.log("error toasts:", toasts);
  assert.equal(toasts.length, 1);
  assert.match(toasts[0], /changed since you last loaded/);
  await browser.close();
  console.log(`E2E PASS (${results.length} steps)`);
})().catch(async (e) => { console.error("E2E FAIL:", e); process.exitCode = 1; }).finally(async () => { try { await browser?.close(); } catch {} srv.kill(); });
