import assert from 'node:assert/strict';
import { inspectRpf, extractRpfEntry } from '../rpf-engine.js';
const count=2, names=new TextEncoder().encode('root.bin\0');
const buf=new ArrayBuffer(512+3), v=new DataView(buf), b=new Uint8Array(buf);
v.setUint32(0,0x52504637,true);v.setUint32(4,count,true);v.setUint32(8,names.length,true);v.setUint32(12,0x4e45504f,true);
// Root directory record.
v.setUint32(16,0,true);v.setUint32(20,0x7fffff00,true);v.setUint32(24,1,true);v.setUint32(28,1,true);
// Standard uncompressed file at block 1.
v.setBigUint64(32,1n<<40n,true);v.setUint32(40,3,true);v.setUint32(44,0,true);
b.set(names,48);b.set([0x5a,0x59,0x4e],512);
const parsed=inspectRpf(buf);assert.equal(parsed.entryCount,2);assert.equal(parsed.rows[0].path,'root.bin');assert.equal(parsed.encryptionName,'OPEN');
const out=await extractRpfEntry(buf,parsed.rows[0]);assert.deepEqual([...new Uint8Array(out)],[0x5a,0x59,0x4e]);
const bad=buf.slice(0);new DataView(bad).setUint32(0,0,true);assert.throws(()=>inspectRpf(bad),/Not a valid RPF7/);
console.log('RPF engine tests passed: header, directory tree, plain extraction, invalid magic.');
import { buildRpf } from '../rpf-engine.js';
const rebuilt=buildRpf([{path:'data/config.dat',data:new TextEncoder().encode('hello config')},{path:'readme.txt',data:new TextEncoder().encode('hello')}]);
const round=inspectRpf(rebuilt);assert.equal(round.rows.length,3);assert.deepEqual(round.rows.map(x=>x.path).sort(),['data','data/config.dat','readme.txt']);
const config=round.rows.find(x=>x.path==='data/config.dat');assert.equal(new TextDecoder().decode(await extractRpfEntry(rebuilt,config)),'hello config');
assert.throws(()=>buildRpf([{path:'../escape',data:new Uint8Array()}]),/Invalid archive path/);
assert.throws(()=>buildRpf([{path:'a',data:new Uint8Array()},{path:'A',data:new Uint8Array()}]),/Duplicate archive path/);
console.log('RPF writer tests passed: create, nested directories, extraction round-trip, path traversal and duplicate rejection.');
import { deflateRawSync } from 'node:zlib';
const compressedText=new TextEncoder().encode('compress-me-'.repeat(80));
const compressedBuf=new ArrayBuffer(512+deflateRawSync(compressedText).length), cv=new DataView(compressedBuf), cb=new Uint8Array(compressedBuf);
cv.setUint32(0,0x52504637,true);cv.setUint32(4,2,true);cv.setUint32(8,8,true);cv.setUint32(12,0,true);
cv.setUint32(16,0,true);cv.setUint32(20,0x7fffff00,true);cv.setUint32(24,1,true);cv.setUint32(28,1,true);
const compressed=deflateRawSync(compressedText);cv.setBigUint64(32,(1n<<40n)|(BigInt(compressed.length)<<16n),true);cv.setUint32(40,compressedText.length,true);cb.set(new TextEncoder().encode('x.bin\0\0\0'),48);cb.set(compressed,512);
const cp=inspectRpf(compressedBuf);assert.equal(cp.rows[0].compressed,true);assert.deepEqual(new Uint8Array(await extractRpfEntry(compressedBuf,cp.rows[0])),compressedText);
console.log('RPF compressed-entry extraction test passed.');

// RSC7 resource entries must survive a rebuild as raw bytes with their resource size metadata.
import { extractAllArchiveEntries, buildRpf as buildRpfResource } from '../rpf-engine.js';
const resourcePayload = new Uint8Array([0x52,0x53,0x43,0x37,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
const resourceArchive = buildRpfResource([{path:'models/test.ydr',data:resourcePayload,resource:true,system:0x1000,graphics:0x2000}]);
const resourceParsed = inspectRpf(resourceArchive);
const resourceRow = resourceParsed.rows.find(x=>x.type==='file');
assert.equal(resourceRow.resource,true);
const resourceEntries = await extractAllArchiveEntries(resourceArchive,resourceParsed);
assert.equal(resourceEntries[0].resource,true);
assert.equal(resourceEntries[0].system,0x1000);
assert.equal(resourceEntries[0].graphics,0x2000);
assert.deepEqual([...resourceEntries[0].data],[...resourcePayload]);
const resourceRebuilt = buildRpfResource(resourceEntries);
const resourceRoundTrip = inspectRpf(resourceRebuilt);
const resourceAgain = await extractAllArchiveEntries(resourceRebuilt,resourceRoundTrip);
assert.deepEqual([...resourceAgain[0].data],[...resourcePayload]);
assert.equal(resourceAgain[0].system,0x1000);
assert.equal(resourceAgain[0].graphics,0x2000);
console.log('RPF resource passthrough test passed: raw RSC7 bytes and system/graphics metadata preserved.');


// Nested archive payloads can be extracted and parsed independently.
const childArchive = buildRpfResource([{path:'inside.txt',data:new TextEncoder().encode('nested-ok')}]);
const parentArchive = buildRpfResource([{path:'nested/child.rpf',data:new Uint8Array(childArchive)}]);
const parentParsed = inspectRpf(parentArchive);
const parentFiles = await extractAllArchiveEntries(parentArchive,parentParsed);
const nestedFile = parentFiles.find(x=>x.path==='nested/child.rpf');
assert.ok(nestedFile);
const nestedParsed = inspectRpf(nestedFile.data.buffer.slice(nestedFile.data.byteOffset,nestedFile.data.byteOffset+nestedFile.data.byteLength));
assert.ok(nestedParsed.rows.some(x=>x.path==='inside.txt'));
console.log('Nested RPF test passed: nested archive payload extracted and independently parsed.');
