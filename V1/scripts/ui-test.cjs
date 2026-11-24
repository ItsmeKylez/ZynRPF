// M2 browser tests (Playwright, Chromium). Usage: node scripts/ui-test.cjs <port> <dataDir> <good.rpf> <big.rpf> <trunc.rpf> [million.rpf]
const { chromium } = require("playwright");
const fs = require("fs");
const [port, dataDir, good, big, trunc, mil] = process.argv.slice(2);
const base = `http://127.0.0.1:${port}`;
let pass = 0, fail = 0;
const check = (name, ok, extra = "") => { if (ok) pass++; else fail++; console.log(`${ok ? "PASS" : "FAIL"} ${name}${ok ? "" : " " + extra}`); };
const axeSrc = fs.readFileSync(process.env.AXE_PATH, "utf8");
const csp = [];

async function newPage(browser) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 }, acceptDownloads: true });
  const page = await ctx.newPage();
  page.on("console", (m) => { if (/Content Security Policy/i.test(m.text())) csp.push(m.text()); });
  page.on("pageerror", (e) => csp.push("pageerror: " + e.message));
  return { ctx, page };
}
async function upload(page, file, timeout = 120000) {
  await page.setInputFiles("#file", file);
  await page.waitForSelector('[aria-label="Archive entries"], [role=alert]', { timeout });
}
async function axeScan(page, label) {
  await page.evaluate(axeSrc);
  const r = await page.evaluate(async () => await window.axe.run(document, { runOnly: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"] }));
  check(`axe ${label}: no violations`, r.violations.length === 0, JSON.stringify(r.violations.map((v) => [v.id, v.impact, v.nodes.length, v.nodes[0]?.target])));
}

(async () => {
  const browser = await chromium.launch();
  // ---- boot + a11y of empty state ----
  let { ctx, page } = await newPage(browser);
  await page.goto(base);
  await page.waitForSelector("h2:has-text('Upload an archive')");
  check("boot: title and upload panel", (await page.title()) === "RPF Editor");
  check("boot: html lang=en", (await page.getAttribute("html", "lang")) === "en");
  check("boot: session cookie is HttpOnly", (await ctx.cookies()).some((c) => c.httpOnly));
  await axeScan(page, "empty state");

  // ---- upload good archive ----
  await upload(page, good);
  check("upload: archive grid appears", await page.locator('[aria-label="Archive entries"]').isVisible());
  const countText = await page.locator("text=/\\d+ entries/").first().innerText();
  check("browse: entry count shown", /\d+ entries/.test(countText), countText);
  check("browse: rows rendered", (await page.locator('[role=row][id^=row-]').count()) > 0);
  await axeScan(page, "archive view");

  // ---- navigate: keyboard + double click + breadcrumb ----
  const rowsBefore = await page.locator('[role=row][id^=row-]').first().innerText();
  await page.locator('[aria-label="Archive entries"]').focus();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(500);
  const crumbs = await page.locator('nav[aria-label="Folder path"] li').count();
  check("navigate: Enter opens a folder (breadcrumb grows)", crumbs >= 2, rowsBefore);
  await page.getByRole("button", { name: "Up one folder" }).click();
  await page.waitForTimeout(400);
  check("navigate: up returns to root", (await page.locator('nav[aria-label="Folder path"] li').count()) === 1);
  await page.locator('[role=row][id^=row-]').first().dblclick();
  await page.waitForTimeout(500);
  check("navigate: double click opens folder", (await page.locator('nav[aria-label="Folder path"] li').count()) >= 2);
  // sort
  await page.getByRole("button", { name: "Sort by Size" }).click();
  await page.waitForTimeout(400);
  check("sort: size sort marks column", await page.getByRole("button", { name: "Sort by Size" }).innerText().then((t) => /▲|▼/.test(t)));

  // ---- select file, detail + hex ----
  const fileRow = page.locator('[role=row][id^=row-]').filter({ hasNotText: "📁" }).first();
  await fileRow.click();
  await page.waitForSelector("pre.hex");
  const hex0 = await page.locator("pre.hex").innerText();
  check("hex: first line starts at offset 00000000", hex0.startsWith("00000000"), hex0.slice(0, 30));
  const detail = await page.locator("section[aria-labelledby=detail-h]").innerText();
  check("detail: shows stored size and offset", /Stored size/.test(detail) && /Offset in archive/.test(detail));
  const off = Number(/Offset in archive\s*(\d+)/.exec(detail)?.[1]);
  const stored = Number(/\((\d+) B\)/.exec(detail)?.[1]);
  const buf = fs.readFileSync(good);
  const firstBytes = Array.from(buf.subarray(off, off + 4)).map((b) => b.toString(16).toUpperCase().padStart(2, "0")).join(" ");
  check("hex: first bytes equal the file bytes", stored === 0 || hex0.includes(firstBytes), `${firstBytes} vs ${hex0.slice(0, 40)}`);
  if (stored > 4096) {
    await page.getByRole("button", { name: "Next page" }).click();
    await page.waitForFunction(() => document.querySelector("pre.hex")?.textContent?.startsWith("00001000"));
    check("hex: next page starts at 00001000", true);
    await page.getByLabel("Go to offset").fill("0x20");
    await page.getByRole("button", { name: "Go" }).click();
    await page.waitForFunction(() => document.querySelector("pre.hex")?.textContent?.startsWith("00000000"));
    check("hex: go to offset returns to page 0", true);
  }
  await page.getByLabel("Go to offset").fill("zzz");
  await page.getByRole("button", { name: "Go" }).click();
  check("hex: invalid offset shows message", await page.locator("#hex-bad").isVisible());
  // entry download equals file slice
  if (stored > 0 && stored <= 64 * 1024 * 1024) {
    const [dl] = await Promise.all([page.waitForEvent("download"), page.getByRole("button", { name: "Download entry bytes" }).click()]);
    const p = await dl.path();
    check("download: entry bytes equal archive slice", Buffer.compare(fs.readFileSync(p), buf.subarray(off, off + stored)) === 0);
  }
  await axeScan(page, "detail and hex");

  // ---- search ----
  const name = (await page.locator(".entry-name").innerText()).trim();
  const term = name.slice(0, Math.max(3, Math.min(6, name.length)));
  await page.getByLabel("Search by name").fill(term);
  await page.getByRole("button", { name: "Search" }).click();
  await page.waitForSelector("ul.hits li");
  check("search: results listed", (await page.locator("ul.hits li").count()) > 0);
  await page.locator("ul.hits li button").first().click();
  await page.waitForSelector('[aria-label="Archive entries"]');
  check("search: result opens its folder and shows detail", await page.locator(".entry-name").isVisible());
  await page.getByLabel("Search by name").fill("no-such-name-zzzz");
  await page.getByRole("button", { name: "Search" }).click();
  await page.waitForSelector("text=No entries match your search.");
  check("search: empty result message", true);

  // ---- download whole archive: byte equality ----
  await page.getByRole("button", { name: "Clear search" }).click();
  const [dl2] = await Promise.all([page.waitForEvent("download"), page.getByRole("link", { name: "Download archive" }).click()]);
  check("download: archive bytes equal original", Buffer.compare(fs.readFileSync(await dl2.path()), buf) === 0);

  // ---- reload keeps the session ----
  await page.reload();
  await page.waitForSelector('[aria-label="Archive entries"]');
  check("reload: archive is still open after a page reload", true);

  // ---- cross-session isolation ----
  const vid = await page.locator("select option").first().getAttribute("value");
  const other = await newPage(browser);
  await other.page.goto(base);
  await other.page.waitForSelector("h2:has-text('Upload an archive')");
  const status = await other.page.evaluate(async (v) => (await fetch(`/api/v1/versions/${v}/entries`)).status, vid);
  check("isolation: other session gets 404 for the version", status === 404, String(status));
  check("isolation: other session sees no archive", (await other.page.locator('[aria-label="Archive entries"]').count()) === 0);
  await other.ctx.close();

  // ---- delete my data ----
  await page.getByRole("button", { name: "Delete my data" }).click();
  await page.getByRole("button", { name: "Yes, delete everything" }).click();
  await page.waitForSelector("h2:has-text('Upload an archive')");
  check("delete data: UI returns to empty state", (await page.locator('[aria-label="Archive entries"]').count()) === 0);
  await page.waitForTimeout(500);
  const left = fs.readdirSync(dataDir).filter((f) => !f.startsWith("."));
  check("delete data: server directories removed", left.length === 0, left.join(","));
  await ctx.close();

  // ---- interruption: transient failure retried, hard failure then Resume ----
  ({ ctx, page } = await newPage(browser));
  await page.goto(base);
  await page.waitForSelector("h2:has-text('Upload an archive')");
  let aborted = 0;
  await page.route(/\/chunks\/3$/, (r) => (aborted++ < 1 ? r.abort() : r.continue()));
  let hard = true;
  await page.route(/\/chunks\/6$/, (r) => (hard ? r.abort() : r.continue()));
  await page.setInputFiles("#file", big);
  await page.waitForSelector("[role=alert]", { timeout: 60000 });
  check("resume: transient chunk failure was retried (chunk 3 sent twice)", aborted >= 2);
  const alertText = await page.locator("[role=alert]").innerText();
  check("resume: hard failure shows message, next step and code", /next/i.test(alertText) && /Error code/.test(alertText), alertText);
  hard = false;
  await page.getByRole("button", { name: "Resume" }).click();
  await page.waitForSelector('[aria-label="Archive entries"]', { timeout: 60000 });
  check("resume: upload completes after Resume", true);
  await page.unrouteAll();
  const resumedOpt = await page.locator("select option").first().innerText();
  // pause / resume
  await page.getByRole("button", { name: "Delete this workspace" }).click();
  await page.waitForSelector("h2:has-text('Upload an archive')");
  await page.route(/\/chunks\/\d+$/, async (r) => { await new Promise((x) => setTimeout(x, 150)); await r.continue(); });
  await page.setInputFiles("#file", big);
  await page.getByRole("button", { name: "Pause" }).click({ timeout: 10000 });
  await page.waitForSelector("text=Upload paused.");
  check("pause: shows paused state", true);
  await page.getByRole("button", { name: "Resume" }).click();
  await page.waitForSelector('[aria-label="Archive entries"]', { timeout: 60000 });
  check("pause: resume completes", true);
  const pausedOpt = await page.locator("select option").first().innerText();
  check("resume: resumed and paused uploads give the same entry count", resumedOpt.replace(/^.*\(/, "") === pausedOpt.replace(/^.*\(/, ""), resumedOpt + " vs " + pausedOpt);
  await ctx.close();

  // ---- hostile upload ----
  ({ ctx, page } = await newPage(browser));
  await page.goto(base);
  await page.waitForSelector("h2:has-text('Upload an archive')");
  await page.setInputFiles("#file", trunc);
  await page.waitForSelector("[role=alert]", { timeout: 30000 });
  const t = await page.locator("[role=alert]").innerText();
  check("hostile: failed job shows title, message, next step and code", /could not be read/i.test(t) && /What to do next/.test(t) && /Error code/.test(t), t);
  await axeScan(page, "error state");
  await ctx.close();

  // ---- English: no non-ASCII visible text except symbols used in UI ----
  // (static audit is a separate script)

  // ---- 1M entries ----
  if (mil) {
    ({ ctx, page } = await newPage(browser));
    await page.goto(base);
    await page.waitForSelector("h2:has-text('Upload an archive')");
    const t0 = Date.now();
    await upload(page, mil, 300000);
    const up = Date.now() - t0;
    const c = await page.locator("text=/\\d+ entries/").first().innerText();
    check("1M: archive opens", /\d+ entries/.test(c), c);
    console.log(`INFO 1M upload+parse in browser: ${up} ms; ${c}`);
    // find a folder with many children: open first folder
    await page.locator('[aria-label="Archive entries"]').focus();
    const t1 = Date.now();
    await page.keyboard.press("End");
    await page.waitForFunction(() => document.querySelector('[role=row][aria-selected=true]') !== null);
    await page.waitForTimeout(300);
    const rows = await page.locator('[role=row][id^=row-]').count();
    check("1M: End key renders only a window of rows (virtualized)", rows > 0 && rows < 120, String(rows));
    console.log(`INFO End key to render: ${Date.now() - t1} ms, DOM rows: ${rows}`);
    const loading = await page.locator('[role=row][id^=row-] >> text=Loading…').count();
    check("1M: no rows stuck loading after End", loading === 0, String(loading));
    await ctx.close();
  }
  await browser.close();
  check("CSP: no violations or page errors in any run", csp.length === 0, csp.join(" | "));
  console.log(`\n${pass} passed, ${fail} failed`);
  process.exit(fail ? 1 : 0);
})().catch((e) => { console.error("TEST CRASH", e); process.exit(2); });
