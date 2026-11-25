// Unit tests for the pure helpers (Node) and the XML check (headless Chromium, real DOMParser). Run: node scripts/test-textcheck.mjs
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { countMatches, replaceAll, looksLikeXml } from "../web/src/textcheck.ts";
assert.equal(countMatches("aAa", "a", true), 2); assert.equal(countMatches("aAa", "a", false), 3); assert.equal(countMatches("aaaa", "aa", true), 2); assert.equal(countMatches("x", "", true), 0);
assert.deepEqual(replaceAll("a-A-a", "a", "b", true), { text: "b-A-b", count: 2 });
assert.deepEqual(replaceAll("a-A-a", "a", "b", false), { text: "b-b-b", count: 3 });
assert.deepEqual(replaceAll("a.b", ".", "$&", true), { text: "a$&b", count: 1 }); // literal, no patterns
assert.equal(replaceAll("abc", "", "x", true).count, 0);
assert.equal(looksLikeXml('<?xml version="1.0"?><a/>'), true); assert.equal(looksLikeXml("  <root>"), true); assert.equal(looksLikeXml("hello"), false);
const require = createRequire(import.meta.url);
const TOOLS = process.env.TOOLS || "/opt/npm-tools/node_modules";
const esbuild = require(TOOLS + "/esbuild");
const { chromium } = require(TOOLS + "/playwright");
const out = await esbuild.build({ entryPoints: ["web/src/textcheck.ts"], bundle: true, format: "iife", globalName: "tc", write: false });
const b = await chromium.launch(); const page = await b.newPage();
await page.setContent("<html></html>"); await page.addScriptTag({ content: out.outputFiles[0].text });
const r = await page.evaluate(() => [tc.checkXml('<?xml version="1.0"?><a><b/></a>'), tc.checkXml("<a><b></a>"), tc.checkXml("<a>")]);
await b.close();
assert.equal(r[0], null); assert.equal(typeof r[1], "string"); assert.equal(typeof r[2], "string");
console.log("textcheck tests passed; sample error:", r[1].slice(0, 80));
