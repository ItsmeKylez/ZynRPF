// M3 browser tests (Playwright, Chromium). Usage: node scripts/ui-test-m3.cjs <port> <small.rpf>
// Covers: edit toolbar, change list, undo/redo, hostile name, check, save with progress and report,
// versions list, one-writer rule across two tabs, keyboard reachability. (No axe scan: axe-core is not installed here.)
const { chromium } = require("playwright");
const fs = require("fs");
const [port, small] = process.argv.slice(2);
const base = `http://127.0.0.1:${port}`;
let pass = 0, fail = 0;
const check = (name, ok, extra = "") => { if (ok) pass++; else fail++; console.log(`${ok ? "PASS" : "FAIL"} ${name}${ok ? "" : " " + extra}`); };
const errors = [];

(async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 1000 }, acceptDownloads: true });
  const page = await ctx.newPage();
  page.on("pageerror", (e) => errors.push("pageerror: " + e.message));
  page.on("console", (m) => { if (/Content Security Policy/i.test(m.text())) errors.push(m.text()); });
  await page.goto(base);
  await page.waitForSelector("h2:has-text('Upload an archive')");
  await page.setInputFiles("#file", small);
  await page.waitForSelector('[aria-label="Archive entries"]');
  await page.waitForSelector("h2:has-text('Edit and save')");
  await page.waitForFunction(() => !document.body.innerText.includes("Checking whether you can edit"));
  check("edit panel shown, this tab is the writer", !(await page.locator("text=read-only").count()) && await page.getByRole("button", { name: "New folder" }).isEnabled());

  // open the folder and select a file
  await page.locator('[role=row][id^=row-]').first().dblclick();
  await page.waitForSelector('[role=row][id^=row-]:has-text("f0000.bin")');
  await page.locator('[role=row][id^=row-]:has-text("f0000.bin")').click();
  await page.waitForSelector("text=Selected: /d00000/f0000.bin");
  check("selected path and folder shown", await page.locator("text=New items go into: /d00000").isVisible());
  check("Replace enabled for a file, Undo disabled at start", await page.getByRole("button", { name: "Replace contents" }).isEnabled() && await page.getByRole("button", { name: "Undo" }).isDisabled());

  // rename
  await page.getByRole("button", { name: "Rename" }).click();
  await page.getByLabel("New name").fill("renamed.bin");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.waitForSelector('ol[aria-label="List of pending changes"] li');
  check("rename appears in the change list", (await page.locator('ol[aria-label="List of pending changes"] li').first().innerText()) === "Rename /d00000/f0000.bin to renamed.bin");

  // hostile: duplicate name -> error box with check code
  await page.getByRole("button", { name: "New folder" }).click();
  await page.getByLabel("Folder name").fill("F0001.BIN");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.waitForSelector("[role=alert]");
  const alert = await page.locator("[role=alert]").first().innerText();
  check("duplicate name is refused with its code", /already exists/.test(alert) && /EDIT_NAME_EXISTS/.test(alert), alert);
  await page.getByRole("button", { name: "Cancel" }).click();
  check("rejected change was not logged", (await page.locator('ol[aria-label="List of pending changes"] li').count()) === 1);

  // new folder + add file with content
  await page.getByRole("button", { name: "New folder" }).click();
  await page.getByLabel("Folder name").fill("added");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.waitForFunction(() => document.querySelectorAll('ol[aria-label="List of pending changes"] li').length === 2);
  const content = Buffer.from("browser test content\n".repeat(50));
  await page.getByRole("button", { name: "Add file" }).click();
  await page.setInputFiles('input[type=file]:not(#file)', { name: "note.txt", mimeType: "text/plain", buffer: content });
  check("file name field is prefilled from the chosen file", (await page.getByLabel("Name inside the archive").inputValue()) === "note.txt");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.waitForFunction(() => document.querySelectorAll('ol[aria-label="List of pending changes"] li').length === 3);
  check("add file logged with its size", /Add file note\.txt \(1\.03 KiB\) to \/d00000/.test(await page.locator('ol[aria-label="List of pending changes"] li').nth(2).innerText()));

  // undo / redo
  await page.getByRole("button", { name: "Undo" }).click();
  await page.waitForSelector("li.undone");
  check("undo marks the last change as undone and enables Redo", (await page.locator("li.undone").innerText()).includes("(undone)") && await page.getByRole("button", { name: "Redo" }).isEnabled());
  await page.getByRole("button", { name: "Redo" }).click();
  await page.waitForFunction(() => document.querySelectorAll("li.undone").length === 0);
  check("redo restores it", true);

  // check changes
  await page.getByRole("button", { name: "Check changes" }).click();
  await page.waitForSelector('[aria-label="Validation report"]');
  check("check changes shows a passing report with a plan", /All checks passed/.test(await page.locator('[aria-label="Validation report"]').innerText()) && /would have 7 entries/.test(await page.locator('[aria-label="Validation report"]').innerText()));

  // keyboard reachability of the toolbar
  await page.getByRole("button", { name: "Rename" }).focus();
  await page.keyboard.press("Tab");
  check("toolbar buttons are reachable by keyboard", await page.evaluate(() => document.activeElement && document.activeElement.tagName === "BUTTON"));

  // second tab: read-only
  const page2 = await ctx.newPage();
  await page2.goto(base);
  await page2.waitForSelector("h2:has-text('Edit and save')");
  await page2.waitForSelector("text=read-only");
  check("second tab is read-only with a clear message", /being edited in another tab/.test(await page2.locator("p.notice").innerText()) && await page2.getByRole("button", { name: "New folder" }).isDisabled());
  check("second tab can still see the pending changes", (await page2.locator('ol[aria-label="List of pending changes"] li').count()) === 3);

  // save
  await page.getByRole("button", { name: "Save as new version" }).click();
  await page.waitForSelector("text=Saved as a new version.", { timeout: 20000 });
  check("save succeeds with a validation report", /All checks passed|0 errors/.test(await page.locator('[aria-label="Validation report"]').first().innerText()));
  await page.waitForFunction(() => document.querySelectorAll('ul[aria-label="Saved versions"] li').length === 2, null, { timeout: 10000 });
  check("two versions listed, newest from edits", /Saved from edits/.test(await page.locator('ul[aria-label="Saved versions"]').innerText()));
  check("pending changes cleared after save", await page.locator("text=No pending changes.").isVisible());
  // the browser now shows the new version: root has the renamed file and the new folder
  await page.waitForSelector('[role=row][id^=row-]');
  await page.locator('[role=row][id^=row-]').first().dblclick();
  await page.waitForSelector('[role=row][id^=row-]:has-text("note.txt")');
  const names = (await page.locator('[role=row][id^=row-]').allInnerTexts()).join("|");
  check("new version shows the renamed file, the added folder and the added file", names.includes("renamed.bin") && names.includes("added") && names.includes("note.txt") && !names.includes("f0000.bin"), names);
  const sel = await page.locator("select").inputValue();
  check("the new version is selected", sel.length > 10);

  // download the new version and check the magic bytes
  const [dl] = await Promise.all([page.waitForEvent("download"), page.locator('ul[aria-label="Saved versions"] li').nth(1).getByRole("link", { name: "Download archive" }).click()]);
  const p = await dl.path();
  const buf = fs.readFileSync(p);
  check("downloaded new version is an RPF7 file", buf.readUInt32LE(0) === 0x52504637 && buf.length > 1000);

  // standalone check of a version
  await page.locator('ul[aria-label="Saved versions"] li').nth(1).getByRole("button", { name: "Check" }).click();
  await page.waitForFunction(() => document.querySelectorAll('[aria-label="Validation report"]').length >= 1);
  check("standalone version check reports no errors", /All checks passed/.test(await page.locator('[aria-label="Validation report"]').last().innerText()));

  // closing tab 1 releases the lease, so tab 2 becomes the writer (lease check runs every 10 s)
  await page.close();
  await page2.waitForFunction(() => !document.body.innerText.includes("being edited in another tab"), null, { timeout: 25000 });
  check("after the writer tab closes, the other tab can edit", await page2.getByRole("button", { name: "New folder" }).isEnabled());

  check("no CSP violations or page errors", errors.length === 0, errors.join(" | "));
  await browser.close();
  console.log(`\n${fail === 0 ? "M3 UI PASS" : "M3 UI FAIL"}: ${pass} checks passed, ${fail} failed`);
  process.exit(fail === 0 ? 0 : 1);
})().catch((e) => { console.error(e); process.exit(2); });
