// English audit for the web UI (D-002, D-032). Exit code 1 on any finding.
import fs from "node:fs";
import path from "node:path";
const root = new URL("../web/", import.meta.url).pathname;
const src = path.join(root, "src");
const files = fs.readdirSync(src).filter((f) => /\.(ts|tsx|css)$/.test(f));
const findings = [];
// 1. non-ASCII scan, allowing a small set of symbols used as UI glyphs.
const allowed = new Set(["…", "▲", "▼", "▶", "📁", "—"]);
for (const f of [...files.map((f) => path.join(src, f)), path.join(root, "index.html")]) {
  const t = fs.readFileSync(f, "utf8");
  for (const [i, line] of t.split("\n").entries()) for (const ch of line) if (ch.charCodeAt(0) > 127 && !allowed.has(ch)) findings.push(`${path.basename(f)}:${i + 1} non-ASCII character U+${ch.codePointAt(0).toString(16)}`);
}
// 2. catalogue coverage: every m.key used exists, every key is used.
const cat = fs.readFileSync(path.join(src, "messages.ts"), "utf8");
const keys = [...cat.matchAll(/^\s{2}(\w+):/gm)].map((x) => x[1]);
const code = files.filter((f) => f.endsWith(".tsx") || f === "api.ts" || f.endsWith(".ts") && f !== "messages.ts").map((f) => fs.readFileSync(path.join(src, f), "utf8")).join("\n");
for (const [, k] of code.matchAll(/\bm\.(\w+)/g)) if (!keys.includes(k)) findings.push(`catalogue is missing key "${k}"`);
const dynamic = ["detected_"]; // keys looked up dynamically
for (const k of keys) if (!new RegExp(`\\bm\\.${k}\\b`).test(code) && !dynamic.some((d) => k.startsWith(d))) findings.push(`catalogue key "${k}" is never used`);
// 3. no literal UI text in markup: JSX text nodes and user-facing attributes must not contain letters outside {expressions}.
for (const f of files.filter((f) => f.endsWith(".tsx"))) {
  const t = fs.readFileSync(path.join(src, f), "utf8");
  for (const [i, line] of t.split("\n").entries()) {
    const stripped = line.replace(/\{[^{}]*(\{[^{}]*\}[^{}]*)*\}/g, "");
    if (/>[^<>{}=]*[A-Za-z]{2,}[^<>{}]*</.test(stripped) && !/^\s*(import|\/\/)/.test(line) && !/=>|&&|\?|<[A-Z]\w*\s*\/?>\s*$/.test(stripped.replace(/<[^>]+>/g, "")) ) findings.push(`${f}:${i + 1} literal text in markup: ${line.trim().slice(0, 80)}`);
    for (const [, a, v] of line.matchAll(/\b(aria-label|placeholder|title|alt)="([^"]*[A-Za-z]{2,}[^"]*)"/g)) findings.push(`${f}:${i + 1} literal ${a}="${v}"`);
  }
}
// 4. spell check against a small list of common US/UK differences and typos.
const uk = /\b(colour|behaviour|favourite|licence|catalogue name|organis\w+|analys(e|ed|ing)|cancelled|grey|centre|metre|initialis\w+)\b/i;
for (const [i, line] of cat.split("\n").entries()) if (uk.test(line)) findings.push(`messages.ts:${i + 1} UK spelling: ${line.trim().slice(0, 80)}`);
if (findings.length) { console.log(findings.join("\n")); console.log(`FAIL: ${findings.length} finding(s)`); process.exit(1); }
console.log(`PASS: ${files.length} files scanned, ${keys.length} catalogue keys, no findings`);
