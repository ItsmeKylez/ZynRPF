// visualsettings.dat editor UI test against the real server. Usage: node scripts/ui-test-vs.cjs <port> <vs.rpf>
// Make the archive: cargo run --release -p rpf-core --example gen -- vs.rpf vs   (synthetic file, not a game file)
const { chromium } = require("playwright");
const [port, file] = process.argv.slice(2);
let pass = 0, fail = 0;
const check = (n, ok, x = "") => { ok ? pass++ : fail++; console.log(`${ok ? "PASS" : "FAIL"} ${n}${ok ? "" : " " + x}`); };
(async () => {
  const b = await chromium.launch(); const page = await (await b.newContext({ viewport: { width: 1280, height: 1800 } })).newPage(); const errs = [];
  page.on("pageerror", (e) => errs.push(e.message));
  await page.goto(`http://127.0.0.1:${port}`);
  await page.setInputFiles("#file", file);
  await page.waitForSelector('[aria-label="Archive entries"]');
  const open = async () => {
    await page.locator('[aria-label="Archive entries"]').getByText("visualsettings.dat", { exact: true }).first().click();
    await page.getByRole("button", { name: "Edit visual settings" }).click();
    await page.getByText("8 settings, 2 color groups").waitFor();
  };
  await open();
  check("search filters settings", (await page.getByPlaceholder(/Search settings/).fill("bloom"), await page.getByLabel("Value of bloom.threshold").count() === 1));
  const val = page.getByLabel("Value of bloom.threshold");
  await val.fill("0.5"); await val.press("Enter");
  check("queue button counts the change", await page.getByRole("button", { name: "Queue changes (1)" }).count() === 1);
  await page.getByPlaceholder(/Search settings/).fill("DamageOverlay");
  const dur = page.getByLabel("Value of misc.DamageOverlay.Duration");
  await dur.fill("900"); await dur.press("Enter");
  check("two changes counted", await page.getByRole("button", { name: "Queue changes (2)" }).count() === 1);
  await page.getByPlaceholder(/Search settings/).fill("");
  await page.getByRole("button", { name: "Sky and clouds" }).click();
  check("preset shows color group", await page.getByText("sun", { exact: false }).count() > 0);
  await page.getByRole("button", { name: /Tint 1 visible color/ }).click();
  check("tint changes the color", await page.getByRole("button", { name: "Queue changes (5)" }).count() === 1);
  await page.getByRole("button", { name: "Queue changes (5)" }).click();
  await page.getByText(/Changes queued/).waitFor();
  await page.getByRole("button", { name: /^Save/ }).first().click();
  await page.waitForTimeout(1500);
  check("saved version appears as a tab", await page.getByRole("tab").count() >= 2);
  await open();
  await page.getByPlaceholder(/Search settings/).fill("bloom");
  check("saved archive shows 0.5", await page.getByLabel("Value of bloom.threshold").inputValue() === "0.5");
  await page.getByPlaceholder(/Search settings/).fill("DamageOverlay");
  check("saved archive shows 900f (f suffix kept)", await page.getByLabel("Value of misc.DamageOverlay.Duration").inputValue() === "900f");
  check("no page errors", errs.length === 0, errs.join("; "));
  await b.close();
  console.log(`${pass} passed, ${fail} failed`); process.exit(fail ? 1 : 0);
})();
