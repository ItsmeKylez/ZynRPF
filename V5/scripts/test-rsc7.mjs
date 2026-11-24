// Unit test for web/src/rsc7.ts (run: node scripts/test-rsc7.mjs). Uses Node's built-in type stripping.
import assert from "node:assert/strict";
import { parseRsc7, sizeFromFlags } from "../web/src/rsc7.ts";
const hdr = (magic, ver, s, g) => { const b = new Uint8Array(16), v = new DataView(b.buffer); v.setUint32(0, magic, true); v.setUint32(4, ver, true); v.setUint32(8, s, true); v.setUint32(12, g, true); return b; };
// one 0x200 block: s0 bit (bit 27) set, shift 0
assert.equal(sizeFromFlags(1 << 27), 0x200);
assert.equal(sizeFromFlags((1 << 27) | 1), 0x400);            // base shift 1 doubles
assert.equal(sizeFromFlags(1 << 24), 0x200 * 8);              // s3 counts 8 blocks
assert.equal(sizeFromFlags(3 << 17), 0x200 * (3 << 4));       // s4 field
assert.equal(sizeFromFlags(0), 0);
assert.equal(sizeFromFlags(0xFFFFFFFF) > 0, true);
const ok = parseRsc7(hdr(0x37435352, 13, 1 << 27, 1 << 26), 1 << 27, 1 << 26);
assert.deepEqual([ok.magic, ok.version, ok.sysSize, ok.gfxSize, ok.flagsMatchEntry], [true, 13, 0x200, 0x400, true]);
assert.equal(parseRsc7(hdr(0x37435352, 13, 5, 6), 7, 6).flagsMatchEntry, false);
assert.equal(parseRsc7(hdr(0x37435352, 13, 5, 6), null, null).flagsMatchEntry, null);
assert.equal(parseRsc7(hdr(0x12345678, 1, 0, 0), 0, 0).magic, false);
assert.equal(parseRsc7(new Uint8Array(15), 0, 0), null);
// bytes "RSC7" on disk must give the magic
assert.equal(parseRsc7(new Uint8Array([0x52,0x53,0x43,0x37,0,0,0,0,0,0,0,0,0,0,0,0]), 0, 0).magic, true);
console.log("rsc7 tests passed");
