// Drop-anywhere test: a file dropped on the page header (outside the drop zone) starts an upload. Usage: node scripts/ui-test-m5a-drop.cjs <port> <small.rpf>
const { chromium } = require("playwright"); const fs = require("fs");
const [port, small] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch(); const page = await (await b.newContext()).newPage();
  await page.goto(`http://127.0.0.1:${port}`); await page.waitForSelector("#file", { state: "attached" });
  const bytes = fs.readFileSync(small).toString("base64");
  const dt = await page.evaluateHandle(([b64, name]) => { const bin = atob(b64); const u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i); const d = new DataTransfer(); d.items.add(new File([u], name)); return d; }, [bytes, "dropped.rpf"]);
  await page.dispatchEvent("h1", "dragenter", { dataTransfer: dt });
  const overlay = await page.locator("text=Drop the RPF file anywhere").count();
  await page.dispatchEvent("h1", "drop", { dataTransfer: dt });
  await page.waitForSelector('[role=tree]', { timeout: 30000 });
  const ok = overlay === 1;
  console.log(`${ok ? "PASS" : "FAIL"} overlay shown while dragging; PASS drop on header opened the archive`);
  await b.close(); process.exit(ok ? 0 : 1);
})().catch((e) => { console.error(e); process.exit(1); });
