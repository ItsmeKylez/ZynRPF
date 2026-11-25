// Particle editor UI test against the real server. Usage: node scripts/ui-test-ypt.cjs <port> <ypt.rpf>
// Make the archive: cargo run --release -p rpf-core --example gen -- ypt.rpf ypt   (synthetic dictionary, not a game file)
const { chromium } = require("playwright");
const [port, file] = process.argv.slice(2);
let pass = 0, fail = 0;
const check = (n, ok, x = "") => { ok ? pass++ : fail++; console.log(`${ok ? "PASS" : "FAIL"} ${n}${ok ? "" : " " + x}`); };
(async () => {
  const b = await chromium.launch(); const page = await (await b.newContext({ viewport: { width: 1280, height: 1600 } })).newPage(); const errs = [];
  page.on("pageerror", (e) => errs.push(e.message));
  await page.goto(`http://127.0.0.1:${port}`);
  await page.setInputFiles("#file", file);
  await page.waitForSelector('[aria-label="Archive entries"]');
  await page.locator('[aria-label="Archive entries"]').getByText("test.ypt").first().click();
  await page.getByRole("button", { name: "Edit particle effects" }).click();
  await page.getByText("1 effects, 0 emitters, 0 particle settings").waitFor();
  check("effects are listed", await page.getByRole("heading", { name: /eff1/ }).count() === 1);
  // change the second keyframe opacity by hand
  const input = page.getByRole("spinbutton", { name: /eff1 .* time 1\.00 Opacity/ }).first();
  check("keyframe input found", await input.count() === 1);
  await input.fill("0.75");
  check("queue button counts the change", await page.getByRole("button", { name: "Queue changes (1)" }).count() === 1);
  // bulk: density has no effect here, size neither; colour hue shift on a tint property should change values
  await page.getByRole("button", { name: "Apply to matching effects" }).click();
  check("neutral bulk settings change nothing", await page.getByText("No value in the matching effects fits these settings.").count() === 1);
  await page.getByRole("button", { name: "Queue changes (1)" }).click();
  await page.getByText(/Changes queued/).waitFor();
  check("operation is pending", await page.getByText(/1 pending/i).count() > 0 || await page.getByText(/pending/i).count() > 0);
  await page.getByRole("button", { name: /^Save/ }).first().click();
  await page.waitForTimeout(1500);
  const tabs = await page.getByRole("tab").count();
  check("saved version appears as a tab", tabs >= 2, `tabs=${tabs}`);
  await page.locator('[aria-label="Archive entries"]').getByText("test.ypt").first().click();
  await page.getByRole("button", { name: "Edit particle effects" }).click();
  await page.getByText("1 effects, 0 emitters, 0 particle settings").waitFor();
  const v = await page.getByRole("spinbutton", { name: /eff1 .* time 1\.00 Opacity/ }).first().inputValue();
  check("saved archive shows 0.75", Number(v) === 0.75, v);
  check("no page errors", errs.length === 0, errs.join("; "));
  await b.close();
  console.log(`${pass} passed, ${fail} failed`); process.exit(fail ? 1 : 0);
})();
