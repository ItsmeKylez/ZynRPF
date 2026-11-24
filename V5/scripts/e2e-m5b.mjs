// M5b checks: reading compressed files (basis=uncompressed). Usage: node scripts/e2e-m5b.mjs <port> <compressed.rpf>
// Make the archive with: cargo run --release -p rpf-core --example gen -- compressed.rpf compressed
// Server needs no special settings.
import fs from 'node:fs';
const [port, file] = [process.argv[2], process.argv[3]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };
const data = fs.readFileSync(file);
let cookie = '', csrf = '';
async function req(method, path, { body, headers = {}, raw } = {}) {
  const h = { ...headers };
  if (cookie) h.Cookie = cookie;
  if (csrf && method !== 'GET') h['X-CSRF-Token'] = csrf;
  let b = body;
  if (body && !raw && typeof body === 'object') { b = JSON.stringify(body); h['Content-Type'] = 'application/json'; }
  const r = await fetch(base + path, { method, headers: h, body: b });
  const buf = Buffer.from(await r.arrayBuffer());
  let json = null; try { json = JSON.parse(buf.toString()); } catch {}
  return { status: r.status, headers: r.headers, json, buf };
}
const s0 = await req('POST', '/sessions/anonymous');
cookie = s0.headers.getSetCookie()[0].split(';')[0]; csrf = s0.json.csrf_token;
const w = (await req('POST', '/workspaces', { body: { name: 'm5b' } })).json;
const st = (await req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'c.rpf', size: data.length } })).json;
const cs = st.chunk_size;
for (let i = 0; i < st.chunk_count; i++) await req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: data.subarray(i * cs, (i + 1) * cs), raw: true });
const done = await req('POST', `/uploads/${st.upload_id}/complete`);
let job = done.json;
for (let i = 0; i < 100; i++) { job = (await req('GET', `/jobs/${done.json.job_id}`)).json; if (job.state === 'succeeded' || job.state === 'failed') break; await new Promise(r => setTimeout(r, 50)); }
ok(job.state === 'succeeded', `parse job succeeded (got ${job.state})`);
const vid = done.json.version_id;
const list = (await req('GET', `/versions/${vid}/entries`)).json.entries;
const by = n => list.find(e => e.name === n);
ok(list.length === 3 && by('bomb.bin') && by('fixed.txt') && by('stored.txt'), 'root lists the three compressed files');
const FIXED = 'hello hello hello, RPF editor! hello hello\n', STORED = 'line one\nline two\nline three\n';
const get = (n, q, headers) => req('GET', `/versions/${vid}/entries/${by(n).id}/bytes${q}`, { headers });
const d = (await req('GET', `/versions/${vid}/entries/${by('fixed.txt').id}`)).json;
ok(d.compressed === true && d.uncompressed_size === FIXED.length, 'fixed.txt is reported as compressed with the plain size');
let r = await get('fixed.txt', '?basis=uncompressed');
ok(r.status === 200 && r.buf.toString() === FIXED, `fixed.txt (real fixed-Huffman data) decompresses (status ${r.status})`);
r = await get('stored.txt', '?basis=uncompressed');
ok(r.status === 200 && r.buf.toString() === STORED, 'stored.txt (stored blocks) decompresses');
r = await get('fixed.txt', '?basis=uncompressed', { Range: 'bytes=6-10' });
ok(r.status === 206 && r.buf.toString() === 'hello' && r.headers.get('content-range') === `bytes 6-10/${FIXED.length}`, 'Range works on the decompressed bytes');
r = await get('fixed.txt', '?basis=stored');
ok(r.status === 200 && r.buf.length === d.stored_size && r.buf.length !== FIXED.length, 'basis=stored still returns the compressed bytes');
r = await get('fixed.txt', '');
ok(r.status === 200 && r.buf.length === d.stored_size, 'default basis is still stored');
r = await get('bomb.bin', '?basis=uncompressed');
ok(r.status === 422 && r.json && r.json.error.code === 'inflate_failed', `data larger than the declared size is refused (got ${r.status} ${r.json && r.json.error && r.json.error.code})`);
r = await get('fixed.txt', '?basis=nonsense');
ok(r.status === 422 && r.json.error.code === 'basis_not_supported', 'unknown basis is refused');
console.log(`\n${fail === 0 ? 'M5B E2E PASS' : 'M5B E2E FAIL'}: ${pass} checks passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
