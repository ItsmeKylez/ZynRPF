// Texture editor UI test against the real server. Usage: node scripts/ui-test-ytd.cjs <port> <ytd.rpf>
// Make the archive: cargo run --release -p rpf-core --example gen -- ytd.rpf ytd   (synthetic dictionary, not a game file)
const { chromium } = require("playwright");
const zlib = require("zlib");
const [port, file] = process.argv.slice(2);
let pass = 0, fail = 0;
const check = (n, ok, x = "") => { ok ? pass++ : fail++; console.log(`${ok ? "PASS" : "FAIL"} ${n}${ok ? "" : " " + x}`); };
const crcT = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc = (b) => { let c = 0xffffffff; for (const x of b) c = crcT[(c ^ x) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
const chunk = (type, data) => { const b = Buffer.alloc(12 + data.length); b.writeUInt32BE(data.length, 0); b.write(type, 4, "ascii"); data.copy(b, 8); b.writeUInt32BE(crc(b.subarray(4, 8 + data.length)), 8 + data.length); return b; };
function solidPng(w, h, r, g, bl) {
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6;
  const raw = Buffer.alloc(h * (w * 4 + 1)); for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { const o = y * (w * 4 + 1) + 1 + x * 4; raw[o] = r; raw[o + 1] = g; raw[o + 2] = bl; raw[o + 3] = 255; }
  return Buffer.concat([Buffer.from("89504e470d0a1a0a", "hex"), chunk("IHDR", ihdr), chunk("IDAT", zlib.deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}
(async () => {
  const b = await chromium.launch(); const page = await (await b.newContext({ viewport: { width: 1280, height: 2000 } })).newPage(); const errs = [];
  page.on("pageerror", (e) => errs.push(e.message));
  await page.goto(`http://127.0.0.1:${port}`);
  await page.setInputFiles("#file", file);
  await page.waitForSelector('[aria-label="Archive entries"]');
  const open = async () => {
    await page.locator('[aria-label="Archive entries"]').getByText("test.ytd", { exact: true }).first().click();
    await page.getByRole("button", { name: "View and edit textures" }).click();
    await page.getByText("2 textures.").waitFor();
  };
  // average colour of the large preview, via a canvas in the page
  const bigColour = async () => page.evaluate(async () => {
    const img = document.querySelector(".tex-big img");
    if (!img) return null;
    await img.decode();
    const c = document.createElement("canvas"); c.width = img.naturalWidth; c.height = img.naturalHeight;
    const x = c.getContext("2d"); x.drawImage(img, 0, 0);
    const d = x.getImageData(0, 0, c.width, c.height).data; const s = [0, 0, 0];
    for (let i = 0; i < d.length; i += 4) { s[0] += d[i]; s[1] += d[i + 1]; s[2] += d[i + 2]; }
    const n = d.length / 4; return [s[0] / n, s[1] / n, s[2] / n, img.naturalWidth];
  });
  await open();
  check("both textures listed", await page.getByRole("listitem").count() >= 2);
  await page.waitForFunction(() => document.querySelectorAll(".tex-thumb img").length === 2, null, { timeout: 8000 }).then(() => check("both thumbnails loaded", true), () => check("both thumbnails loaded", false));
  await page.getByRole("button", { name: /red_dxt1/ }).click();
  await page.waitForSelector(".tex-big img");
  let c = await bigColour();
  check("large preview is red", c && c[0] > 250 && c[1] < 5 && c[2] < 5 && c[3] === 32, JSON.stringify(c));
  await page.getByLabel("Level").selectOption("1");
  await page.waitForTimeout(500);
  c = await bigColour();
  check("level 1 preview is 16 pixels wide", c && c[3] === 16, JSON.stringify(c));
  await page.getByLabel("Level").selectOption("0");
  // recolour red by +120 degrees
  await page.getByLabel("Hue (degrees)").fill("120");
  await page.getByRole("button", { name: "Recolor this texture" }).click();
  await page.getByText(/1 texture recolored and queued/).waitFor();
  // replace the gradient texture with a solid blue image
  await page.getByRole("button", { name: /grad_dxt5/ }).click();
  await page.getByLabel("Hue (degrees)").fill("0");
  await page.getByLabel("Replace this texture with an image").setInputFiles({ name: "blue.png", mimeType: "image/png", buffer: solidPng(128, 128, 0, 0, 255) });
  await page.getByText(/Texture replaced and queued/).waitFor();
  await page.getByRole("button", { name: /^Save/ }).first().click();
  await page.waitForTimeout(2000);
  check("saved version appears as a tab", await page.getByRole("tab").count() >= 2);
  await open();
  await page.getByRole("button", { name: /red_dxt1/ }).click();
  await page.waitForFunction(() => document.querySelector(".tex-big img")?.naturalWidth === 32);
  c = await bigColour();
  check("saved red texture is green", c && c[1] > 200 && c[0] < 60, JSON.stringify(c));
  await page.getByRole("button", { name: /grad_dxt5/ }).click();
  await page.waitForFunction(() => document.querySelector(".tex-big img")?.naturalWidth === 64);
  c = await bigColour();
  check("saved gradient texture is blue and still 64 wide", c && c[2] > 240 && c[0] < 15 && c[3] === 64, JSON.stringify(c));
  // export link works (PNG download is triggered)
  const [dl] = await Promise.all([page.waitForEvent("download"), page.getByRole("button", { name: "Export as PNG" }).click()]);
  check("PNG export downloads grad_dxt5.png", dl.suggestedFilename() === "grad_dxt5.png", dl.suggestedFilename());
  check("no page errors", errs.length === 0, errs.join("; "));
  await b.close();
  console.log(`${pass} passed, ${fail} failed`); process.exit(fail ? 1 : 0);
})();
