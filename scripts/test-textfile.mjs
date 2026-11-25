import assert from "node:assert/strict";
import { decodeText, encodeText } from "../web/src/textfile.ts";
const enc = (s) => new TextEncoder().encode(s);
let d = decodeText(enc("a\r\nb\r\n")); assert.deepEqual([d.text, d.eol, d.bom], ["a\nb\n", "\r\n", false]);
assert.deepEqual([...encodeText("a\nb\nc\n", d)], [...enc("a\r\nb\r\nc\r\n")]);
d = decodeText(new Uint8Array([0xef,0xbb,0xbf,...enc("x\n")])); assert.deepEqual([d.bom, d.eol, d.text], [true, "\n", "x\n"]);
assert.deepEqual([...encodeText("y\n", d)], [0xef,0xbb,0xbf,...enc("y\n")]);
assert.equal(decodeText(new Uint8Array([65,0,66])), null);          // NUL
assert.equal(decodeText(new Uint8Array([0xff,0xfe,65])), null);     // invalid UTF-8
assert.equal(decodeText(enc("a\rb")), null);                        // lone CR
const u = "héllo ✓\n"; assert.equal(new TextDecoder().decode(encodeText(decodeText(enc(u)).text, decodeText(enc(u)))), u);
const mixed = "a\r\nb\nc\n"; d = decodeText(enc(mixed)); assert.equal(d.eol, "\n");
console.log("textfile tests passed");
