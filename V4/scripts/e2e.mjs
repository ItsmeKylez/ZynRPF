// End-to-end API test. Usage: node scripts/e2e.mjs <port> <dataDir> <archive.rpf (3 files x 2 MiB)> <bigindex.rpf (50k entries)>
// Server must run with CHUNK_SIZE_BYTES=1048576 SESSION_QUOTA_BYTES=20971520 RATE_READS_PER_MIN=200 RATE_WRITES_PER_MIN=100
import fs from 'node:fs';
import crypto from 'node:crypto';
const [port, dataDir, goodPath, bigPath] = [process.argv[2], process.argv[3], process.argv[4], process.argv[5]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };

class Client {
  constructor() { this.cookie = ''; this.csrf = ''; }
  async req(method, path, { body, headers = {}, raw, noCsrf, noCookie } = {}) {
    const h = { ...headers };
    if (!noCookie && this.cookie) h.Cookie = this.cookie;
    if (!noCsrf && this.csrf && method !== 'GET') h['X-CSRF-Token'] = this.csrf;
    let b = body;
    if (body && !raw && typeof body === 'object') { b = JSON.stringify(body); h['Content-Type'] = 'application/json'; }
    const r = await fetch(base + path, { method, headers: h, body: b });
    const buf = Buffer.from(await r.arrayBuffer());
    let json = null; try { json = JSON.parse(buf.toString()); } catch {}
    return { status: r.status, headers: r.headers, json, buf };
  }
  async start() {
    const r = await this.req('POST', '/sessions/anonymous', { noCsrf: true });
    const sc = r.headers.getSetCookie()[0];
    this.cookie = sc.split(';')[0]; this.csrf = r.json.csrf_token;
    return r;
  }
}
async function upload(c, wsid, file, extra = {}) {
  const data = fs.readFileSync(file);
  const s = await c.req('POST', `/workspaces/${wsid}/uploads`, { body: { filename: 'test.rpf', size: data.length, ...extra } });
  if (s.status !== 201) return { start: s };
  const cs = s.json.chunk_size;
  for (let i = 0; i < s.json.chunk_count; i++) {
    const r = await c.req('PUT', `/uploads/${s.json.upload_id}/chunks/${i}`, { body: data.subarray(i * cs, (i + 1) * cs), raw: true });
    if (r.status !== 204) return { start: s, chunkFail: r };
  }
  const done = await c.req('POST', `/uploads/${s.json.upload_id}/complete`);
  let job = done.json;
  for (let i = 0; i < 100 && done.status === 202; i++) {
    const j = await c.req('GET', `/jobs/${done.json.job_id}`); job = j.json;
    if (job.state === 'succeeded' || job.state === 'failed') break;
    await new Promise(r => setTimeout(r, 50));
  }
  return { start: s, done, job, data };
}

console.log('# security basics');
const a = new Client();
ok((await a.req('GET', '/me', { noCookie: true })).status === 401, 'no cookie -> 401');
const s1 = await a.start();
ok(s1.status === 201 && s1.headers.get('set-cookie').includes('HttpOnly') && s1.headers.get('set-cookie').includes('SameSite=Lax'), 'cookie flags');
ok(s1.headers.get('x-content-type-options') === 'nosniff' && s1.headers.get('content-security-policy').includes("default-src 'self'") && s1.headers.get('x-request-id'), 'security headers + request id');
ok((await a.req('POST', '/workspaces', { body: { name: 'x' }, noCsrf: true })).status === 403, 'missing CSRF -> 403');
ok((await a.req('POST', '/workspaces', { body: { name: 'x' }, headers: { 'X-CSRF-Token': 'wrong' }, noCsrf: true })).status === 403, 'wrong CSRF -> 403');
const bj = await a.req('POST', '/workspaces', { body: '{bad', raw: true });
ok(bj.status === 400 && bj.json.error.code === 'invalid_json', 'malformed JSON -> 400 invalid_json');
const lim = await a.req('GET', '/limits');
ok(lim.json.max_entries === 1000000 && lim.json.max_range_bytes === 1048576, '/limits');

console.log('# workspace + upload + resume');
const w = (await a.req('POST', '/workspaces', { body: { name: 'My mods' } })).json;
ok(!!w.id, 'workspace created');
const w2 = await a.req('POST', '/workspaces', { body: { name: 'second' } });
ok(w2.status === 409 && w2.json.error.code === 'limit_exceeded', 'second workspace -> limit_exceeded');
const good = fs.readFileSync(goodPath);
// manual flow to test resume and wrong lengths
const st = (await a.req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'good.rpf', size: good.length } })).json;
ok(st.chunk_size === 1048576 && st.chunk_count === Math.ceil(good.length / 1048576), 'start response');
const bad1 = await a.req('PUT', `/uploads/${st.upload_id}/chunks/0`, { body: good.subarray(0, 100), raw: true });
ok(bad1.status === 400 && bad1.json.error.code === 'length_mismatch', 'wrong chunk length rejected');
const bad2 = await a.req('PUT', `/uploads/${st.upload_id}/chunks/999`, { body: good.subarray(0, 100), raw: true });
ok(bad2.status === 400 && bad2.json.error.code === 'invalid_index', 'chunk index out of range rejected');
for (const i of [0, 1, 3]) await a.req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: good.subarray(i * 1048576, (i + 1) * 1048576), raw: true });
const inc = await a.req('POST', `/uploads/${st.upload_id}/complete`);
ok(inc.status === 409 && inc.json.error.code === 'upload_incomplete', 'complete with missing chunks -> 409');
const stat = (await a.req('GET', `/uploads/${st.upload_id}`)).json;
ok(JSON.stringify(stat.received) === '[0,1,3]', 'status lists received chunks');
for (const i of [2, 4, 5, 6].filter(i => i < st.chunk_count)) await a.req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: good.subarray(i * 1048576, (i + 1) * 1048576), raw: true });
const comp = await a.req('POST', `/uploads/${st.upload_id}/complete`);
ok(comp.status === 202, 'complete -> 202');
let job; for (let i = 0; i < 100; i++) { job = (await a.req('GET', `/jobs/${comp.json.job_id}`)).json; if (job.state === 'succeeded' || job.state === 'failed') break; await new Promise(r => setTimeout(r, 50)); }
ok(job.state === 'succeeded' && job.progress === 100 && job.version_id === comp.json.version_id, 'parse job succeeded');
const vid = comp.json.version_id;

console.log('# browsing');
const root = (await a.req('GET', `/versions/${vid}/entries`)).json;
ok(root.total === 1 && root.entries[0].kind === 'directory' && root.entries[0].name === 'd00000', 'root lists one folder');
const dir = (await a.req('GET', `/versions/${vid}/entries?parent=${root.entries[0].id}&sort=name&limit=2`)).json;
ok(dir.total === 3 && dir.entries.length === 2 && dir.next_cursor === '2', 'paged listing with cursor');
const dir2 = (await a.req('GET', `/versions/${vid}/entries?parent=${root.entries[0].id}&limit=2&cursor=2&order=desc`)).json;
ok(dir2.entries.length === 1 && dir2.next_cursor === null, 'last page');
const f0 = dir.entries[0];
ok(f0.kind === 'file' && f0.stored_size === 2097152 && f0.compressed === false, 'file metadata');
const det = (await a.req('GET', `/versions/${vid}/entries/${f0.id}`)).json;
ok(det.path === '/d00000/f0000.bin' && det.detected_type === 'binary', 'entry detail path + type');
const pth = (await a.req('GET', `/versions/${vid}/entries/${f0.id}/path`)).json;
ok(pth.chain.length === 3 && pth.chain[0].id === '0' && pth.path === '/d00000/f0000.bin', 'breadcrumb chain');
const sr = (await a.req('GET', `/versions/${vid}/search?q=F0001`)).json;
ok(sr.hits.length === 1 && sr.hits[0].path === '/d00000/f0001.bin', 'search is case-insensitive');
ok((await a.req('GET', `/versions/${vid}/search?q=`)).status === 400, 'empty search rejected');
ok((await a.req('GET', `/versions/${vid}/entries/${root.entries[0].id}/bytes`)).status === 422, 'folder has no bytes');
ok((await a.req('GET', `/versions/${vid}/entries/99999`)).status === 404, 'unknown entry -> 404');
ok((await a.req('GET', `/versions/${vid}/entries?parent=${f0.id}`)).status === 422, 'listing a file -> 422');
ok((await a.req('GET', `/versions/${vid}/entries/..%2f..%2fetc`)).status === 404, 'traversal-looking id -> 404');

console.log('# byte access');
const nr = await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes`);
ok(nr.status === 416 && nr.json.error.code === 'range_required', '>1 MiB without Range -> 416 range_required');
const r1 = await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes`, { headers: { Range: 'bytes=0-1048575' } });
ok(r1.status === 206 && r1.buf.length === 1048576 && r1.buf.equals(good.subarray(f0.offset, f0.offset + 1048576)) && r1.headers.get('content-range') === `bytes 0-1048575/2097152`, '1 MiB range equals file bytes');
const r2 = await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes`, { headers: { Range: 'bytes=2097000-' } });
ok(r2.status === 206 && r2.buf.length === 152 && r2.buf.equals(good.subarray(f0.offset + 2097000, f0.offset + 2097152)), 'open-ended range clamps at entry end');
ok((await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes`, { headers: { Range: 'bytes=0-1048576' } })).status === 416, 'range > 1 MiB -> 416');
ok((await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes`, { headers: { Range: 'bytes=0-1,5-9' } })).status === 416, 'multi-range -> 416');
ok((await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes`, { headers: { Range: 'bytes=-5' } })).status === 416, 'suffix range -> 416');
ok((await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes`, { headers: { Range: 'bytes=9999999-' } })).status === 416, 'start past end -> 416');
ok((await a.req('GET', `/versions/${vid}/entries/${f0.id}/bytes?basis=decompressed`, { headers: { Range: 'bytes=0-9' } })).status === 422, 'decompressed basis -> 422 (not in M1)');
const dl = await a.req('GET', `/versions/${vid}/download`);
ok(dl.status === 200 && dl.buf.equals(good) && dl.headers.get('content-disposition').startsWith('attachment'), 'download returns the original bytes');
const dlr = await a.req('GET', `/versions/${vid}/download`, { headers: { Range: 'bytes=100-199' } });
ok(dlr.status === 206 && dlr.buf.equals(good.subarray(100, 200)), 'download supports Range');

console.log('# limits and quota');
const huge = await a.req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'x.rpf', size: 2 ** 40 } });
ok(huge.status === 413 && huge.json.error.code === 'limit_exceeded' && huge.json.error.details.max === 1073741824, 'over upload limit -> 413 limit_exceeded with details');
const q1 = await a.req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'x.rpf', size: 15 * 1048576 } });
ok(q1.status === 413 && q1.json.error.code === 'quota_exceeded', 'over quota -> 413 quota_exceeded');
ok((await a.req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'x.rpf', size: 0 } })).status === 400, 'zero size rejected');
ok((await a.req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'a\u0000b', size: 5 } })).status === 400, 'control char in filename rejected');
const me = (await a.req('GET', '/me')).json;
ok(me.usage_bytes === good.length && me.quota_bytes === 20971520, '/me usage');

console.log('# hostile uploads');
const trunc = '/tmp/m1_trunc.rpf'; fs.writeFileSync(trunc, good.subarray(0, 3000));
const h1 = await upload(a, w.id, trunc);
ok(h1.job.state === 'failed' && h1.job.error.code === 'DATA_OUT_OF_BOUNDS' && /extends past the end/.test(h1.job.error.message) && h1.job.error.next_step, 'truncated archive -> job failed with code, message, next step');
const garbage = '/tmp/m1_garbage.rpf'; fs.writeFileSync(garbage, crypto.randomBytes(5000));
const h2 = await upload(a, w.id, garbage);
ok(h2.job.state === 'failed' && h2.job.error.code === 'RPF_HEADER_BAD_MAGIC', 'random bytes -> RPF_HEADER_BAD_MAGIC');
const vlist = (await a.req('GET', `/workspaces/${w.id}/versions`)).json.versions;
ok(vlist.filter(v => v.state === 'failed').length === 2 && vlist.filter(v => v.state === 'ready').length === 1, 'failed versions are marked failed');
const failedV = h1.job.version_id;
const fe = await a.req('GET', `/versions/${failedV}/entries`);
ok(fe.status === 409, 'browsing a failed version -> 409');
const files = fs.readdirSync(`${dataDir}/${w.id}`);
ok(files.filter(f => f.startsWith('v-')).length === 1 && !files.some(f => f.endsWith('.part')), 'failed uploads removed from disk');
const hh = await upload(a, w.id, trunc, { sha256: '0'.repeat(64) });
ok(hh.job.state === 'failed' && hh.job.error.code === 'UPLOAD_HASH_MISMATCH', 'wrong sha256 -> UPLOAD_HASH_MISMATCH');
const hok = await upload(a, w.id, goodPath, { sha256: crypto.createHash('sha256').update(good).digest('hex') });
ok(hok.job.state === 'succeeded', 'correct sha256 accepted');

console.log('# isolation between sessions');
const b = new Client(); await b.start();
for (const p of [`/workspaces/${w.id}`, `/workspaces/${w.id}/versions`, `/versions/${vid}/entries`, `/versions/${vid}/entries/0`, `/versions/${vid}/download`, `/jobs/${comp.json.job_id}`, `/uploads/${st.upload_id}`, `/versions/${vid}/search?q=f`]) {
  const r = await b.req('GET', p); ok(r.status === 404 && r.json.error.code === 'not_found', `foreign GET ${p} -> 404`);
}
ok((await b.req('DELETE', `/workspaces/${w.id}`)).status === 404, 'foreign DELETE workspace -> 404');
ok((await b.req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'x', size: 5 } })).status === 404, 'foreign upload start -> 404');
ok((await b.req('GET', '/workspaces')).json.workspaces.length === 0, 'foreign session lists nothing');

console.log('# large index paging, then delete');
ok((await a.req('DELETE', `/workspaces/${w.id}`)).status === 204, 'delete workspace');
ok(!fs.existsSync(`${dataDir}/${w.id}`), 'workspace directory removed from disk');
ok((await a.req('GET', `/versions/${vid}/entries`)).status === 404, 'versions gone after delete');
const w3 = (await a.req('POST', '/workspaces', { body: { name: 'big' } })).json;
const big = await upload(a, w3.id, bigPath);
ok(big.job.state === 'succeeded', 'large-index archive parsed');
const bv = big.job.version_id;
const t0 = Date.now();
const bl = (await a.req('GET', `/versions/${bv}/entries?parent=1&limit=500`)).json;
ok(bl.entries.length === 500 && bl.next_cursor === '500', 'folder with many children pages at 500');
ok((await a.req('GET', `/versions/${bv}/search?q=f0007&limit=400`)).json.hits.length > 0, 'search over large index');
console.log(`  (large-index list+search took ${Date.now() - t0} ms)`);

console.log('# rate limit');
const c = new Client(); await c.start();
let limited = null; for (let i = 0; i < 260; i++) { const r = await c.req('GET', '/me'); if (r.status === 429) { limited = r; break; } }
ok(limited && limited.json.error.code === 'rate_limited' && limited.headers.get('retry-after'), 'read rate limit -> 429 with Retry-After');

console.log('# sign out deletes data');
ok((await b.req('DELETE', '/sessions/current')).status === 204, 'sign out');
ok((await b.req('GET', '/me')).status === 401, 'session invalid after sign out');

console.log(`\n${fail === 0 ? 'M1 E2E PASS' : 'M1 E2E FAIL'}: ${pass} checks passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
