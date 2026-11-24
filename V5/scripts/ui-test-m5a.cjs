// M5a browser tests (Playwright, Chromium). Usage: node scripts/ui-test-m5a.cjs <port> <small.rpf>
// Covers: lazy tree, tree selection, collapse all, summary panel, keyboard shortcuts, save and download, theme switch.
const { chromium } = require("playwright");
const [port, small] = process.argv.slice(2);
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
  await page.waitForFunction(() => !document.body.innerText.includes("Checking whether you can edit"));
  const tree = page.getByRole("tree", { name: "Archive tree" });
  check("tree shows top-level entries", (await tree.getByRole("treeitem").count()) >= 1);
  const first = tree.getByRole("treeitem").first();
  check("folders start collapsed", (await first.getAttribute("aria-expanded")) === "false");
  await first.getByRole("button", { name: /^Expand/ }).click();
  await page.waitForSelector('[role=tree] [role=treeitem] [role=treeitem]');
  check("expanding loads children lazily", (await first.getAttribute("aria-expanded")) === "true" && (await first.getByRole("treeitem").count()) > 0);
  const child = first.getByRole("treeitem").first();
  await child.locator(".tree-name").click();
  await page.waitForSelector("text=Selected: /");
  check("clicking a tree file selects it for editing", await page.locator("text=/Selected: \\/.+\\/.+/").count() > 0);
  await page.getByRole("button", { name: "Collapse all" }).click();
  await page.waitForSelector('[role=tree] [role=treeitem]');
  check("collapse all folds the tree", (await page.locator('[role=tree] [role=treeitem] [role=treeitem]').count()) === 0);
  await page.getByRole("button", { name: "Expand all" }).click();
  await page.waitForSelector('[role=tree] [role=treeitem] [role=treeitem]');
  check("expand all opens folders (bounded depth)", (await page.locator('[role=tree] [role=treeitem][aria-expanded=true]').count()) >= 1 && /levels/.test(await page.locator("body").innerText()));
  await page.getByRole("button", { name: "Collapse all" }).click();
  await page.waitForFunction(() => document.querySelectorAll('[role=tree] [role=treeitem] [role=treeitem]').length === 0);
  const sum = page.getByRole("region", { name: "Summary" }).or(page.locator("section:has(h2:has-text('Summary'))"));
  check("summary shows file name and entry count", /good\.rpf/.test(await sum.innerText()) && /Entries/.test(await sum.innerText()));
  check("summary says no pending changes", /None/.test(await sum.innerText()));

  // edit, undo and redo with keys, estimate after check, save with Ctrl+S
  await page.getByRole("button", { name: "New folder" }).click();
  await page.locator('input[type=text]').fill("m5a_folder");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.waitForSelector("text=m5a_folder");
  check("summary counts the pending change", /Pending changes\s*1/.test((await sum.innerText()).replace(/\s+/g, " ")));
  await page.getByRole("button", { name: "Check changes" }).click();
  await page.waitForSelector("text=Estimated size after saving");
  await page.waitForFunction(() => /after saving\s*\d/.test((document.querySelector("#sum-h")?.parentElement?.innerText || "").replace(/\s+/g, " ")), null, { timeout: 8000 }).catch(() => {});
  check("estimated size appears after Check changes", /after saving\s*\d/.test((await sum.innerText()).replace(/\s+/g, " ")), (await sum.innerText()).replace(/\s+/g, " "));
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.keyboard.press("Control+z");
  await page.waitForSelector("li.undone");
  check("Ctrl+Z undoes the last change", await page.locator("li.undone").count() === 1);
  await page.keyboard.press("Control+Shift+z");
  await page.waitForFunction(() => document.querySelectorAll("li.undone").length === 0);
  check("Ctrl+Shift+Z redoes it", true);
  await page.keyboard.press("Control+s");
  await page.waitForSelector("text=Saved as a new version", { timeout: 20000 });
  check("Ctrl+S saves a new version", true);

  // Live preview of pending edits in the tree (F-06)
  await page.getByRole("button", { name: "New folder" }).click();
  await page.locator('input[type=text]').fill("preview_dir");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.waitForSelector('[role=tree] >> text=preview_dir');
  check("pending new folder appears in the tree before saving, with the preview note", (await tree.innerText()).includes("preview_dir") && await page.locator("text=The tree shows your pending changes").count() === 1);
  const row = tree.locator(".tree-row", { hasText: "preview_dir" }).first();
  await row.locator(".tree-name").click();
  await page.waitForSelector("text=Selected: /preview_dir");
  check("a pending item can be selected for editing", true);
  await page.getByRole("toolbar", { name: "Edit actions" }).getByRole("button", { name: "Delete" }).click();
  await page.waitForFunction(() => !document.querySelector('[role=tree]')?.textContent?.includes("preview_dir"));
  check("a pending delete removes the item from the tree", true);
  await page.keyboard.press("Control+z");
  await page.waitForSelector('[role=tree] >> text=preview_dir');
  check("undo brings it back in the tree", true);

  // Quick search (F-08)
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  await page.keyboard.press("Control+p");
  const dlg = page.getByRole("dialog", { name: "Quick search" });
  await dlg.waitFor();
  check("Ctrl+P opens the quick search dialog with focus in the box", await page.evaluate(() => document.activeElement?.getAttribute("aria-label") === "Search the whole archive"));
  await dlg.getByRole("combobox").fill("f0");
  await dlg.getByRole("option").first().waitFor();
  check("quick search lists matches", (await dlg.getByRole("option").count()) >= 1);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await dlg.waitFor({ state: "detached" });
  check("Enter picks a result and closes the dialog", await page.locator("text=/Selected: \\/.+/").count() > 0);
  await page.keyboard.press("Control+p"); await dlg.waitFor(); await page.keyboard.press("Escape");
  await dlg.waitFor({ state: "detached" });
  check("Escape closes the dialog", true);

  // Save and download
  await page.getByRole("button", { name: "New folder" }).click();
  await page.locator('input[type=text]').fill("second");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.waitForSelector("text=second");
  const [dl] = await Promise.all([page.waitForEvent("download", { timeout: 20000 }), page.getByRole("button", { name: "Save and download" }).click()]);
  const path = await dl.path(); const head = require("fs").readFileSync(path).subarray(0, 4).toString("latin1");
  check("Save and download delivers an RPF7 file", head === "7FPR");

  await page.selectOption("header select", "dark");
  check("theme switch sets data-theme", (await page.evaluate(() => document.documentElement.getAttribute("data-theme"))) === "dark");
  check("keyboard shortcut bar is listed", await page.getByRole("list", { name: "Keyboard shortcuts" }).count() === 1);
  check("no page errors", errs.length === 0, errs.join("; "));
  await b.close();
  console.log(`\nM5a UI ${fail ? "FAIL" : "PASS"}: ${pass} passed, ${fail} failed`); process.exit(fail ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(1); });
