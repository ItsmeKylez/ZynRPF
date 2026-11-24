// Tabs, recent list and Export all. Usage: node scripts/ui-test-m5a-tabs.cjs <port> <good.rpf> <compressed.rpf>
const { chromium } = require("playwright");
const [port, small, comp] = process.argv.slice(2);
let pass = 0, fail = 0;
const check = (n, ok, x = "") => { ok ? pass++ : fail++; console.log(`${ok ? "PASS" : "FAIL"} ${n}${ok ? "" : " " + x}`); };
(async () => {
  const b = await chromium.launch();
  const ctx = await b.newContext({ viewport: { width: 1280, height: 1100 }, acceptDownloads: true });
  const page = await ctx.newPage(); const errs = [];
  page.on("pageerror", (e) => errs.push(e.message));
  await page.goto(`http://127.0.0.1:${port}`);
  await page.setInputFiles("#file", small);
  await page.waitForSelector('[role=tree]');
  const tabs = () => page.getByRole("tablist", { name: "Open archives" }).getByRole("tab");
  check("one tab after the first upload", (await tabs().count()) === 1 && /good\.rpf/.test(await tabs().first().innerText()));
  await page.getByRole("button", { name: "Upload another archive" }).click();
  await page.setInputFiles("#file", comp);
  await page.waitForFunction(() => document.querySelectorAll('[role=tablist] [role=tab]').length === 2, null, { timeout: 20000 });
  check("second upload opens a second tab and selects it", (await tabs().nth(1).getAttribute("aria-selected")) === "true" && /compressed\.rpf/.test(await tabs().nth(1).innerText()));
  await page.waitForSelector('[aria-label="Archive entries"]');
  check("browser shows the second archive", await page.locator("text=fixed.txt").first().isVisible());
  await tabs().nth(0).click();
  await page.waitForFunction(() => document.querySelector('[role=tab][aria-selected=true]')?.textContent?.includes("good.rpf"));
  await page.waitForFunction(() => !document.querySelector('[aria-label="Archive entries"]')?.textContent?.includes("fixed.txt"), null, { timeout: 10000 }).catch(() => {});
  check("clicking a tab switches archive", (await page.locator('[aria-label="Archive entries"]').getByText("fixed.txt").count()) === 0);
  await tabs().nth(0).focus(); await page.keyboard.press("ArrowRight");
  await page.waitForFunction(() => document.querySelector('[role=tab][aria-selected=true]')?.textContent?.includes("compressed.rpf"));
  check("arrow key moves to the next tab", true);

  // Export all: the link downloads a real zip
  const dl = page.waitForEvent("download");
  await page.getByRole("link", { name: "Export all (ZIP)" }).click();
  const d = await dl;
  check("Export all downloads a .zip", /\.zip$/.test(d.suggestedFilename()), d.suggestedFilename());
  const p = await d.path(); const hdr = require("fs").readFileSync(p).subarray(0, 2).toString();
  check("downloaded file is a zip (PK header)", hdr === "PK");

  // close tab and recent list
  await page.getByRole("button", { name: "Close tab compressed.rpf" }).click();
  await page.waitForFunction(() => document.querySelectorAll('[role=tablist] [role=tab]').length === 1);
  check("closing a tab removes it and selects the neighbour", /good\.rpf/.test(await tabs().first().innerText()) && (await tabs().first().getAttribute("aria-selected")) === "true");
  await page.locator("details.recent summary").click();
  const item = page.locator("details.recent li", { hasText: "compressed.rpf" });
  check("recent list still shows the closed archive", await item.isVisible());
  await item.getByRole("button", { name: "Open in a tab" }).click();
  await page.waitForFunction(() => document.querySelectorAll('[role=tablist] [role=tab]').length === 2);
  check("recent list reopens a closed tab", (await tabs().nth(1).getAttribute("aria-selected")) === "true");
  check("no page errors", errs.length === 0, errs.join("; "));
  await b.close();
  console.log(`${fail === 0 ? "PASS" : "FAIL"} ui-test-m5a-tabs: ${pass} passed, ${fail} failed`);
  process.exit(fail ? 1 : 0);
})().catch((e) => { console.log("FAIL exception", e.message); process.exit(1); });
