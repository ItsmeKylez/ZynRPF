// Unit tests for the particle bulk-edit helpers. Run: node scripts/test-yptbulk.mjs
import assert from "node:assert/strict";
import { applyBulk, changeLines, filterGroups, shiftHue } from "../web/src/yptbulk.ts";
const fr = (offset, value) => ({ time: [0, 0, 0, 0], value, offset });
const groups = [
  { kind: "particle", name: "ptfx_blood", behaviour: "Colour", properties: [{ name: "ptxu_Colour:m_rgbaMinKFP", frames: [fr(100, [1, 0, 0, 0.5])] }] },
  { kind: "emitter", name: "muz_pistol", behaviour: null, properties: [{ name: "ptxEmitterRule:m_spawnRateOverTimeKFP", frames: [fr(200, [10, 0, 0, 0])] }, { name: "ptxEmitterRule:m_sizeScalarKFP", frames: [fr(300, [1, 1, 1, 0])] }] },
];
// hue shift red by 120 degrees -> green, brightness kept, alpha untouched
const g = shiftHue([1, 0, 0], 120);
assert.ok(g[1] > 0.99 && g[0] < 0.01 && g[2] < 0.01, JSON.stringify(g));
const hdr = shiftHue([4, 0, 0], 120);
assert.ok(Math.abs(hdr[1] - 4) < 1e-6, "HDR colours keep their peak");
assert.deepEqual(shiftHue([0.5, 0.5, 0.5], 90), [0.5, 0.5, 0.5], "greys are unchanged");

let p = new Map();
assert.equal(applyBulk(groups, { hue: 120 }, p), 1);
assert.equal(p.get(100)[3], 0.5);
p = new Map();
assert.equal(applyBulk(groups, { density: 0 }, p), 1);
assert.deepEqual(p.get(200), [0, 0, 0, 0], "density 0 stops spawning");
assert.ok(!p.has(300), "size values are not touched by density");
p = new Map();
assert.equal(applyBulk(groups, { size: 2 }, p), 1);
assert.deepEqual(p.get(300), [2, 2, 2, 0]);
p = new Map();
assert.equal(applyBulk(groups, { tint: "#00ff00", strength: 1 }, p), 1);
assert.deepEqual(p.get(100).map((x) => Math.round(x * 100) / 100), [0, 1, 0, 0.5]);
assert.equal(applyBulk(groups, {}, new Map()), 0, "neutral settings change nothing");

assert.equal(filterGroups(groups, "blood").length, 1);
assert.equal(filterGroups(groups, "muz pistol").length, 1);
assert.equal(filterGroups(groups, "colour").length, 1, "behaviour is searched too");
assert.equal(filterGroups(groups, "", "emitter").length, 1);
assert.equal(changeLines(new Map([[100, [0.1, 0.2, 0.3, 0.4]]])), "100 0.1 0.2 0.3 0.4\n");
console.log("yptbulk tests passed");
