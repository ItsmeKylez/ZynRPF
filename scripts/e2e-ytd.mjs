// Texture editor checks (server routes + real save round trip). Usage: node scripts/e2e-ytd.mjs <port> <ytd.rpf>
// Make the archive: cargo run --release -p rpf-core --example gen -- ytd.rpf ytd   (synthetic dictionary, not a game file)
import fs from 'node:fs';
const [port, file] = [process.argv[2], process.argv[3]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };
const data = fs.readFileSync(file);
let cookie = '', csrf = '';
async function req(method, path, { body, raw } = {}) {
  const h = { 'X-Tab-Id': 'tab-ytd00001' };
  if (cookie) h.Cookie = cookie;
  if (csrf && method !== 'GET') h['X-CSRF-Token'] = csrf;
  let b = body;
  if (body && !raw && typeof body === 'object') { b = JSON.stringify(body); h['Content-Type'] = 'application/json'; }
  const r = await fetch(base + path, { method, headers: h, body: b });
  const buf = Buffer.from(await r.arrayBuffer());
  let json = null; try { json = JSON.parse(buf.toString()); } catch {}
  return { status: r.status, headers: r.headers, json, buf };
}
const poll = async (jid) => { for (let i = 0; i < 200; i++) { const j = (await req('GET', `/jobs/${jid}`)).json; if (j.state === 'succeeded' || j.state === 'failed') return j; await new Promise(r => setTimeout(r, 50)); } throw new Error('job timeout'); };
const s0 = await req('POST', '/sessions/anonymous');
cookie = s0.headers.getSetCookie()[0].split(';')[0]; csrf = s0.json.csrf_token;
const w = (await req('POST', '/workspaces', { body: { name: 'ytd' } })).json;
const st = (await req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'p.rpf', size: data.length } })).json;
for (let i = 0; i < st.chunk_count; i++) await req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: data.subarray(i * st.chunk_size, (i + 1) * st.chunk_size), raw: true });
const done = await req('POST', `/uploads/${st.upload_id}/complete`);
const job = await poll(done.json.job_id);
ok(job.state === 'succeeded', 'parse job succeeded');
const v1 = done.json.version_id;
import zlib from 'node:zlib';
// minimal PNG helpers (8-bit RGBA, non-interlaced)
function pngDecode(buf) {
  if (buf.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') throw new Error('not a PNG');
  let o = 8, w = 0, h = 0, idat = [];
  while (o < buf.length) { const n = buf.readUInt32BE(o), type = buf.toString('ascii', o + 4, o + 8), d = buf.subarray(o + 8, o + 8 + n); if (type === 'IHDR') { w = d.readUInt32BE(0); h = d.readUInt32BE(4); if (d[8] !== 8 || d[9] !== 6) throw new Error('not RGBA8'); } if (type === 'IDAT') idat.push(d); o += 12 + n; }
  const raw = zlib.inflateSync(Buffer.concat(idat)), bpp = 4, st = w * 4, out = Buffer.alloc(h * st);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (st + 1)], line = raw.subarray(y * (st + 1) + 1, (y + 1) * (st + 1));
    for (let x = 0; x < st; x++) {
      const a = x >= bpp ? out[y * st + x - bpp] : 0, b = y ? out[(y - 1) * st + x] : 0, c = x >= bpp && y ? out[(y - 1) * st + x - bpp] : 0;
      let v = line[x];
      if (f === 1) v += a; else if (f === 2) v += b; else if (f === 3) v += (a + b) >> 1; else if (f === 4) { const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c); v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      out[y * st + x] = v & 255;
    }
  }
  return { w, h, px: out };
}
const crcT = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc = (b) => { let c = 0xffffffff; for (const x of b) c = crcT[(c ^ x) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
function chunk(type, data) { const b = Buffer.alloc(12 + data.length); b.writeUInt32BE(data.length, 0); b.write(type, 4, 'ascii'); data.copy(b, 8); b.writeUInt32BE(crc(b.subarray(4, 8 + data.length)), 8 + data.length); return b; }
function pngEncodeSolid(w, h, r, g, b) {
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6;
  const raw = Buffer.alloc(h * (w * 4 + 1)); for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { const o = y * (w * 4 + 1) + 1 + x * 4; raw[o] = r; raw[o + 1] = g; raw[o + 2] = b; raw[o + 3] = 255; }
  return Buffer.concat([Buffer.from('89504e470d0a1a0a', 'hex'), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}
const avg = (p) => { const s = [0, 0, 0, 0]; const n = p.px.length / 4; for (let i = 0; i < p.px.length; i++) s[i % 4] += p.px[i]; return s.map(x => x / n); };

const entry = (vid) => req('GET', `/versions/${vid}/entries`).then(r => r.json.entries.find(e => e.name === 'test.ytd'));
const e1 = await entry(v1);
ok(e1 && e1.kind === 'resource', 'test.ytd is a resource entry');
const T = (v, e, p) => `/versions/${v}/entries/${e.id}/textures${p}`;

let r = await req('GET', T(v1, e1, ''));
ok(r.status === 200 && r.json.textures.length === 2 && r.json.embedded === false, 'list returns two textures (' + r.status + ')');
const byName = (j, n) => j.textures.find(t => t.name === n);
const red = byName(r.json, 'red_dxt1'), grad = byName(r.json, 'grad_dxt5');
ok(red && red.width === 32 && red.height === 32 && /DXT1/i.test(red.format) && red.supported, 'red_dxt1 is 32x32 DXT1');
ok(grad && grad.width === 64 && grad.height === 64 && /DXT5/i.test(grad.format) && grad.levels >= 4, 'grad_dxt5 is 64x64 DXT5 with mips');

r = await req('GET', T(v1, e1, `/${red.index}/png`));
let p = r.status === 200 ? pngDecode(r.buf) : null;
ok(p && p.w === 32 && p.h === 32, 'red preview is a 32x32 PNG');
ok(p && avg(p)[0] > 250 && avg(p)[1] < 5 && avg(p)[2] < 5, 'red preview pixels are red: ' + (p && avg(p).map(x => x.toFixed(0))));
r = await req('GET', T(v1, e1, `/${red.index}/png?mip=1`));
p = r.status === 200 ? pngDecode(r.buf) : null;
ok(p && p.w === 16 && p.h === 16, 'mip 1 is 16x16');
r = await req('GET', T(v1, e1, `/${grad.index}/dds`));
ok(r.status === 200 && r.buf.subarray(0, 4).toString() === 'DDS ', 'DDS export starts with the DDS magic');
r = await req('GET', T(v1, e1, '/9/png'));
ok(r.status === 404, 'unknown texture index is 404');
const gradBefore = (await req('GET', T(v1, e1, `/${grad.index}/png`))).buf;

// invalid requests
r = await req('POST', T(v1, e1, '/recolor?indices=0'), {});
ok(r.status === 400, 'recolor without a change is rejected');
r = await req('POST', T(v1, e1, '/recolor?indices=0&hue=999'), {});
ok(r.status === 400, 'out-of-range hue is rejected');
r = await req('POST', T(v1, e1, '/recolor?indices=0&hue=10&tint=zzz'), {});
ok(r.status === 400, 'bad tint is rejected');
r = await req('POST', T(v1, e1, '/recolor?indices=77&hue=10'), {});
ok(r.status === 404, 'recolor of an unknown index is rejected');
r = await req('POST', T(v1, e1, `/${red.index}/replace`), { body: Buffer.from('this is not an image'), raw: true });
ok(r.status === 422 && r.json.error.code === 'image_unreadable', 'garbage image is rejected');
const huge = pngEncodeSolid(8, 8, 1, 2, 3); huge.writeUInt32BE(20000, 16); // claim a 20000 pixel width (CRC now wrong, size check must still refuse or fail cleanly)
r = await req('POST', T(v1, e1, `/${red.index}/replace`), { body: huge, raw: true });
ok(r.status === 422, 'oversized image is refused (' + r.status + ')');
const r0 = await req('GET', `/versions/${v1}/entries`); ok(r0.status === 200, 'server still healthy');

// recolor texture red -> hue +120 (green), queue, save
const colourise = async (v, e, query) => {
  const rr = await req('POST', T(v, e, query), {});
  if (rr.status !== 200) return rr;
  const b = await req('PUT', `/workspaces/${w.id}/blobs`, { body: rr.buf, raw: true });
  const o = await req('POST', `/workspaces/${w.id}/edit/ops`, { body: { version: v, op: 'replace', path: '/test.ytd', blob_id: b.json.blob_id } });
  return { status: o.status, buf: rr.buf };
};
const save = async () => { const sv = await req('POST', `/workspaces/${w.id}/edit/save`); const sj = await poll(sv.json.job_id); return sj; };
let c = await colourise(v1, e1, `/recolor?indices=${red.index}&hue=120`);
ok(c.status === 200 && c.buf.readUInt32LE(0) === 0x37435352, 'recolor returns an RSC7 resource and the replace op is queued');
let sj = await save();
ok(sj.state === 'succeeded' && sj.report.ok === true, 'save 1 clean (' + JSON.stringify(sj.error || sj.report).slice(0, 160) + ')');
const v2 = sj.version_id; const e2 = await entry(v2);
r = await req('GET', T(v2, e2, `/${red.index}/png`));
p = pngDecode(r.buf);
ok(avg(p)[1] > 200 && avg(p)[0] < 60, 'saved red texture is now green: ' + avg(p).map(x => x.toFixed(0)));
ok(Buffer.compare((await req('GET', T(v2, e2, `/${grad.index}/png`))).buf, gradBefore) === 0, 'untouched texture is pixel-identical');
r = await req('GET', T(v2, e2, ''));
ok(r.json.textures.length === 2 && byName(r.json, 'red_dxt1').width === 32, 'size and format unchanged after recolor');

// replace the gradient texture with a solid blue 128x128 PNG (stays 64x64 DXT5)
const blue = pngEncodeSolid(128, 128, 0, 0, 255);
r = await req('POST', T(v2, e2, `/${grad.index}/replace?fit=stretch`), { body: blue, raw: true });
ok(r.status === 200 && r.buf.readUInt32LE(0) === 0x37435352, 'replace returns an RSC7 resource');
const b2 = await req('PUT', `/workspaces/${w.id}/blobs`, { body: r.buf, raw: true });
const o2 = await req('POST', `/workspaces/${w.id}/edit/ops`, { body: { version: v2, op: 'replace', path: '/test.ytd', blob_id: b2.json.blob_id } });
ok(o2.status === 200, 'replace op queued');
sj = await save();
ok(sj.state === 'succeeded' && sj.report.ok === true, 'save 2 clean');
const v3 = sj.version_id; const e3 = await entry(v3);
r = await req('GET', T(v3, e3, ''));
const g3 = byName(r.json, 'grad_dxt5');
ok(g3.width === 64 && g3.height === 64 && /DXT5/i.test(g3.format) && g3.levels === grad.levels, 'replaced texture keeps its size, format and mip count');
p = pngDecode((await req('GET', T(v3, e3, `/${grad.index}/png`))).buf);
ok(avg(p)[2] > 240 && avg(p)[0] < 15 && avg(p)[1] < 15, 'replaced texture is blue: ' + avg(p).map(x => x.toFixed(0)));
p = pngDecode((await req('GET', T(v3, e3, `/${grad.index}/png?mip=3`))).buf);
ok(avg(p)[2] > 240, 'a smaller mip level is blue too');
p = pngDecode((await req('GET', T(v3, e3, `/${red.index}/png`))).buf);
ok(avg(p)[1] > 200, 'earlier recolor still present');
// queued content (base_blob): reads and edits build on a queued replacement, not on the saved file
r = await req('POST', T(v3, e3, `/recolor?indices=${red.index}&hue=-120`), {});
const bb = await req('PUT', `/workspaces/${w.id}/blobs`, { body: r.buf, raw: true });
const qbase = `base_blob=${bb.json.blob_id}`;
p = pngDecode((await req('GET', T(v3, e3, `/${red.index}/png?${qbase}`))).buf);
ok(avg(p)[0] > 200 && avg(p)[1] < 60, 'preview with base_blob shows the queued (red again) texture, not the saved green one');
p = pngDecode((await req('GET', T(v3, e3, `/${grad.index}/png?${qbase}`))).buf);
ok(avg(p)[2] > 240, 'texture untouched by the queued edit still reads correctly through base_blob');
r = await req('POST', T(v3, e3, `/${grad.index}/replace?fit=fill&${qbase}`), { body: pngEncodeSolid(16, 16, 0, 255, 0), raw: true });
const b4 = await req('PUT', `/workspaces/${w.id}/blobs`, { body: r.buf, raw: true });
p = pngDecode((await req('GET', T(v3, e3, `/${red.index}/png?base_blob=${b4.json.blob_id}`))).buf);
ok(avg(p)[0] > 200, 'a second edit built on the queued content keeps the first edit (red texture still red)');
p = pngDecode((await req('GET', T(v3, e3, `/${grad.index}/png?base_blob=${b4.json.blob_id}`))).buf);
ok(avg(p)[1] > 240 && avg(p)[2] < 15, 'and applies the second edit (gradient texture now green)');
r = await req('GET', T(v3, e3, '?base_blob=999'));
ok(r.status === 400, 'unknown base_blob is rejected');
r = await req('GET', T(v3, e3, '?base_blob=abc'));
ok(r.status === 400, 'malformed base_blob is rejected');
const vv = await req('POST', `/versions/${v3}/validate`);
ok(vv.status === 200 && vv.json.ok !== false, 'saved archive validates');
console.log(`${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
