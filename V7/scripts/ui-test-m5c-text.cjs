// Text editor. NEVER RUN (written on a machine without cargo). Usage: node scripts/ui-test-m5c-text.cjs <port> <compressed.rpf>
// Opens the compressed test archive, edits fixed.txt as text, queues the change and checks the pending operation count.
const { chromium } = require("playwright");
const [port, comp] = process.argv.slice(2);
let pass = 0, fail = 0;
const check = (n, ok, x = "") => { ok ? pass++ : fail++; console.log(`${ok ? "PASS" : "FAIL"} ${n}${ok ? "" : " " + x}`); };
(async () => {
  const b = await chromium.launch(); const page = await (await b.newContext({ viewport: { width: 1280, height: 1400 } })).newPage(); const errs = [];
  page.on("pageerror", (e) => errs.push(e.message));
  await page.goto(`http://127.0.0.1:${port}`);
  await page.setInputFiles("#file", comp);
  await page.waitForSelector('[aria-label="Archive entries"]');
  await page.locator('[aria-label="Archive entries"]').getByText("fixed.txt").first().click();
  await page.getByRole("button", { name: "Edit as text" }).click();
  const ta = page.getByRole("textbox", { name: /Contents of fixed\.txt/ });
  await ta.waitFor({ timeout: 10000 });
  const before = await ta.inputValue();
  check("compressed file content is loaded as text", before.length > 0);
  await ta.fill(before + "\nedited by test\n");
  await page.getByRole("button", { name: "Queue change" }).click();
  await page.waitForSelector("text=The change is queued", { timeout: 10000 });
  check("change is queued", true);
  check("no page errors", errs.length === 0, errs.join(";"));
  console.log(`${pass} passed, ${fail} failed`); await b.close(); process.exit(fail ? 1 : 0);
})();
