// M5a browser E2E (visual settings): real zyn-server binary + built web UI + Chromium.
// Prereqs: `npm run build:web`; `cd web && cargo build -p zyn-server --examples`. Run: node web-tests/e2e-m5a-settings.cjs "$PWD" /tmp/m5a-out
// (Playwright must be resolvable, e.g. NODE_PATH=$(npm root -g).)
const { chromium } = require("playwright");
const { spawn, execFileSync } = require("child_process");
const assert = require("assert/strict");
const fs = require("fs");
const path = require("path");
const repo = process.argv[2], out = process.argv[3];
const PORT = 18491;
fs.mkdirSync(out, { recursive: true });
const sample = path.join(out, "sample.rpf");
execFileSync(path.join(repo, "web/target/debug/examples/make_sample_rpf"), [sample]);
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
const VS = "# keep me\r\nlight.intensity 1.5f\r\nlight.color.red 255\r\nlight.color.green 128\r\nlight.color.blue 0\r\n\r\nfog.density 0.25\r\nfog.density 0.30\r\n// and me\r\n";
let browser;
const collected = [];
const results = [];
const step = async (name, fn) => { await fn(); results.push(name); console.log("ok  -", name); };

(async () => {
  for (let i = 0; i < 50; i++) { try { if ((await fetch(`http://127.0.0.1:${PORT}/healthz`)).ok) break; } catch {} await sleep(100); }
  browser = await chromium.launch();
  const page = await (await browser.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
  const problems = [], failed = [], errToasts = [];
  await page.addInitScript(() => {
    window.__err = [];
    new MutationObserver((ms) => ms.forEach((m) => m.addedNodes.forEach((n) => n.classList && n.classList.contains("hata") && window.__err.push(n.textContent)))).observe(document, { childList: true, subtree: true });
  });
  page.on("console", (m) => m.type() === "error" && problems.push(m.text()));
  page.on("pageerror", (e) => problems.push("pageerror: " + e.message));
  page.on("response", (r) => { if (r.status() >= 400) failed.push(`${r.status()} ${r.request().method()} ${new URL(r.url()).pathname}`); });
  const api = (p, init) => page.evaluate(async ([p, init]) => (await fetch("/api/v1" + p, init)).json(), [p, init]);
  const raw = () => page.evaluate(async () => await (await fetch("/api/v1/documents/1/entries/raw?path=visualsettings.dat")).text());
  const summary = () => api("/documents/1");
  const row = (name) => page.locator(".satir", { has: page.locator(".ad", { hasText: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) }) });
  const toast = (t) => page.locator(".bildirim", { hasText: t }).first().waitFor({ timeout: 5000 });
  const waitRaw = (pred, label) => until(async () => pred(await raw())).catch(async () => { throw new Error(`${label}: file is ${JSON.stringify(await raw())}`); });
  const search = (q) => page.locator('input[placeholder^="Ayar ara"]').fill(q);

  await page.goto(`http://127.0.0.1:${PORT}/`);
  await page.locator("input[type=file]").first().setInputFiles(sample);
  await row("content").waitFor();
  await api("/documents/1/entries/text", { method: "POST", headers: { "X-Requested-With": "zyn", "Content-Type": "application/json" }, body: JSON.stringify({ path: "visualsettings.dat", text: VS }) });
  collected.push(...(await page.evaluate(() => window.__err))); await page.reload();
  await row("visualsettings.dat").waitFor();

  await step("settings file opens in the settings editor with counts from the core parser", async () => {
    await row("visualsettings.dat").click();
    await page.locator(".duzenleyici-ust h2 small", { hasText: "5 ayar · 1 renk" }).waitFor();
  });

  await step("edit a duplicated key: every occurrence changes, comments and CRLF survive, one undo step", async () => {
    await search("fog");
    const input = page.locator(".ayar-satiri", { hasText: "density" }).locator("input.girdi");
    await input.fill("0.9");
    await input.press("Enter");
    // (the undo-depth wait below is the synchronisation point)
    await until(() => page.evaluate(async () => (await (await fetch("/api/v1/documents/1")).json()).undoDepth === 2));
    assert.equal(await raw(), VS.replace("0.25", "0.9").replace("0.30", "0.9"));
    await page.locator('button[title^="Geri al"]').click();
    await until(() => page.evaluate(async () => (await (await fetch("/api/v1/documents/1")).json()).undoDepth === 1));
    assert.equal(await raw(), VS);
  });

  await step("float suffix is preserved and the slider path works", async () => {
    await search("intensity");
    const input = page.locator(".ayar-satiri", { hasText: "intensity" }).locator("input.girdi");
    await input.fill("2.25");
    await input.press("Enter");
    await until(() => page.evaluate(async () => (await (await fetch("/api/v1/documents/1/entries/raw?path=visualsettings.dat")).text()).includes("light.intensity 2.25f")));
  });

  await step("colour picker rewrites the three channel keys (0-255 scale)", async () => {
    await search("light");
    await page.locator(".renk-kutu input[type=color]").first().fill("#00ff10");
    await until(() => page.evaluate(async () => { const t = await (await fetch("/api/v1/documents/1/entries/raw?path=visualsettings.dat")).text(); return t.includes("light.color.red 0\r\n") && t.includes("light.color.green 255\r\n") && t.includes("light.color.blue 16\r\n"); }));
  });

  await step("a value the format cannot hold is refused with a clear error and nothing changes", async () => {
    const before = await raw();
    await search("fog");
    const input = page.locator(".ayar-satiri", { hasText: "density" }).locator("input.girdi");
    await input.fill("1 2");
    await input.press("Enter");
    await toast("Invalid setting value");
    assert.equal(await raw(), before);
  });

  await step("revert-all restores every changed setting to its opening value", async () => {
    await search("");
    await page.getByRole("button", { name: /Hepsini geri döndür/ }).click();
    await page.locator(".modal .eylemler button.tehlike, .modal .eylemler button.birincil", { hasText: "Geri döndür" }).first().click();
    await waitRaw((t) => t.includes("light.intensity 1.5f") && t.includes("light.color.green 128"), "revert");
    const t = await raw();
    assert.ok(t.includes("light.color.red 255\r\n") && t.includes("fog.density 0.25\r\n"), t);
  });

  await step("save, reload and reopen: edits persist, original upload is untouched", async () => {
    await search("fog");
    const f = page.locator(".ayar-satiri", { hasText: "density" }).locator("input.girdi");
    await f.fill("0.7");
    await f.press("Enter");
    await until(() => page.evaluate(async () => (await (await fetch("/api/v1/documents/1/entries/raw?path=visualsettings.dat")).text()).includes("fog.density 0.7")));
    await page.locator(".belge-ust button.birincil", { hasText: /Kaydet/ }).click();
    await toast("Saved to your workspace");
    collected.push(...(await page.evaluate(() => window.__err))); await page.reload();
    await row("visualsettings.dat").waitFor();
    assert.equal(await raw(), VS.replace("0.25", "0.7").replace("0.30", "0.7"));
    const fid = (await api("/files"))[0].fileId;
    const orig = await page.evaluate(async (fid) => (await (await fetch(`/api/v1/files/${fid}/download?version=original`)).arrayBuffer()).byteLength, fid);
    assert.equal(orig, fs.statSync(sample).size);
  });

  await step("a non-settings path is refused (no silent success)", async () => {
    const r = await page.evaluate(async () => { const x = await fetch("/api/v1/documents/1/settings", { method: "PUT", headers: { "X-Requested-With": "zyn", "Content-Type": "application/json" }, body: JSON.stringify({ path: "content/nothing.dat", values: [["a", "1"]] }) }); return [x.status, (await x.json()).error.code]; });
    assert.deepEqual(r, [404, "NOT_FOUND"]);
  });

  const toasts = [...collected, ...(await page.evaluate(() => window.__err))];
  console.log("failed requests:", failed, "error toasts:", toasts);
  assert.deepEqual(failed.filter((f) => !/^(400 PUT \/api\/v1\/documents\/1\/settings|404 PUT \/api\/v1\/documents\/1\/settings)$/.test(f)), []);
  assert.deepEqual(problems.filter((p) => !/status of (400|404)/.test(p)), []);
  assert.equal(toasts.length, 1);
  await page.screenshot({ path: path.join(out, "m5a.png") });
  console.log(`E2E PASS (${results.length} steps)`);
})().catch((e) => { console.error("E2E FAIL:", e); process.exitCode = 1; }).finally(async () => { try { await browser?.close(); } catch {} srv.kill(); });
