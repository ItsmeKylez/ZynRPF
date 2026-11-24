// Nested RPF browsing test. Usage: node scripts/e2e-nested.mjs <port> <dataDir> <small.rpf>
// Server env: SESSION_QUOTA_BYTES=3000000 MAX_BLOB_BYTES=1048576 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000 CHUNK_SIZE_BYTES=1048576
// Builds an outer archive containing small.rpf (add_file + save), opens it as a nested version, and checks read-only behaviour.
import fs from 'node:fs';
const [port, , smallPath] = [process.argv[2], process.argv[3], process.argv[4]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };
let cookie = '', csrf = '';
const TAB = 'tab-nested01';
async function req(method, path, { body, raw } = {}) {
  const h = { 'X-Tab-Id': TAB };
  if (cookie) h.Cookie = cookie;
  if (csrf && method !== 'GET') h['X-CSRF-Token'] = csrf;
  let b = body;
  if (body && !raw && typeof body === 'object') { b = JSON.stringify(body); h['Content-Type'] = 'application/json'; }
  const r = await fetch(base + path, { method, headers: h, body: b });
  const buf = Buffer.from(await r.arrayBuffer());
  let json = null; try { json = JSON.parse(buf.toString()); } catch {}
  return { status: r.status, headers: r.headers, json, buf };
}
const sleep = ms => new Promise(r => setTimeout(r, ms));
const poll = async jid => { for (let i = 0; i < 300; i++) { const j = (await req('GET', `/jobs/${jid}`)).json; if (j.state === 'succeeded' || j.state === 'failed') return j; await sleep(30); } throw new Error('job timeout'); };
const find = async (vid, path) => { let id = '0'; for (const p of path.split('/').filter(Boolean)) { const l = (await req('GET', `/versions/${vid}/entries?parent=${id}&limit=500`)).json; const e = l.entries.find(x => x.name === p); if (!e) return null; id = e.id; } return id; };

const s = await req('POST', '/sessions/anonymous'); cookie = s.headers.getSetCookie()[0].split(';')[0]; csrf = s.json.csrf_token;
const ws = (await req('POST', '/workspaces', { body: { name: 'nested' } })).json;
const inner = fs.readFileSync(smallPath);
const up = async data => {
  const st = await req('POST', `/workspaces/${ws.id}/uploads`, { body: { filename: 'outer.rpf', size: data.length } });
  const cs = st.json.chunk_size;
  for (let i = 0; i < st.json.chunk_count; i++) await req('PUT', `/uploads/${st.json.upload_id}/chunks/${i}`, { body: data.subarray(i * cs, (i + 1) * cs), raw: true });
  return poll((await req('POST', `/uploads/${st.json.upload_id}/complete`)).json.job_id);
};
// Outer archive = the small archive plus a copy of itself added as /inner.rpf.
const first = await up(inner);
ok(first.state === 'succeeded', 'base archive parsed');
const v1 = first.version_id;
await req('POST', `/workspaces/${ws.id}/writer`);
const blob = await req('PUT', `/workspaces/${ws.id}/blobs`, { body: inner, raw: true });
ok(blob.status === 201, 'inner archive content stored');
ok((await req('POST', `/workspaces/${ws.id}/edit/ops`, { body: { version: v1, op: 'add_file', parent: '/', name: 'inner.rpf', blob_id: blob.json.blob_id } })).status === 200, 'add_file logged');
const saved = await poll((await req('POST', `/workspaces/${ws.id}/edit/save`)).json.job_id);
ok(saved.state === 'succeeded', 'outer archive saved');
const v2 = saved.version;
const outerId = await find(v2 || saved.version_id, '/inner.rpf');
const outer = v2 || saved.version_id;
ok(outerId !== null, 'inner.rpf present in outer archive');
const det = (await req('GET', `/versions/${outer}/entries/${outerId}`)).json;
ok(det.detected_type === 'nested_rpf', 'detected as nested_rpf');

console.log('# open nested');
const notNested = await find(outer, '/d00000/f0000.bin');
const bad = await req('POST', `/versions/${outer}/entries/${notNested}/open`);
ok(bad.status === 422 && bad.json.error.code === 'not_a_nested_archive', 'plain file is refused');
const dir = await req('POST', `/versions/${outer}/entries/${await find(outer, '/d00000')}/open`);
ok(dir.status === 422, 'folder is refused');
const o = await req('POST', `/versions/${outer}/entries/${outerId}/open`);
ok(o.status === 201 && o.json.version_id && o.json.entry_count === inner.length * 0 + o.json.entry_count, `opened: ${o.status} ${o.buf.toString().slice(0, 120)}`);
const nv = o.json.version_id;
const vs = (await req('GET', `/workspaces/${ws.id}/versions`)).json.versions;
const nrow = vs.find(v => v.id === nv);
ok(nrow && nrow.origin === 'nested' && nrow.state === 'ready' && nrow.size === inner.length, 'listed as ready nested version with the inner size');
const a = (await req('GET', `/versions/${v1}/entries?parent=0&limit=500`)).json.entries.map(e => e.name).sort().join();
const b = (await req('GET', `/versions/${nv}/entries?parent=0&limit=500`)).json.entries.map(e => e.name).sort().join();
ok(a === b && a.length > 0, 'nested root listing equals the original inner archive listing');
const f = await find(nv, '/d00000/f0000.bin');
const x1 = (await req('GET', `/versions/${nv}/entries/${f}/bytes`)).buf;
const x2 = (await req('GET', `/versions/${v1}/entries/${await find(v1, '/d00000/f0000.bin')}/bytes`)).buf;
ok(x1.length > 0 && x1.equals(x2), 'nested file bytes equal the original');
ok((await req('GET', `/versions/${nv}/download`)).buf.equals(inner), 'nested download equals the embedded bytes');

console.log('# read only');
const e = await req('POST', `/workspaces/${ws.id}/edit/ops`, { body: { version: nv, op: 'mkdir', parent: '/', name: 'zz' } });
ok(e.status === 404, `edit on nested version refused (${e.status})`);
const parentBytes = (await req('GET', `/versions/${outer}/download`)).buf;
ok(parentBytes.length > inner.length, 'parent archive still intact');

console.log('# quota');
let last;
for (let i = 0; i < 15; i++) { last = await req('POST', `/versions/${outer}/entries/${outerId}/open`); if (last.status !== 201) break; }
ok(last.status === 413 || last.status === 409, `repeat opens are bounded by quota or version cap (${last.status} ${last.json?.error?.code})`);
console.log(`${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
