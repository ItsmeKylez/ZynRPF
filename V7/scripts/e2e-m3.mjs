// M3 end-to-end test: edit operations, undo/redo, save, versions, writer lock, quota.
// Usage: node scripts/e2e-m3.mjs <port> <dataDir> <small.rpf (gen small.rpf 5 50000)>
// Server env: SESSION_QUOTA_BYTES=3000000 MAX_BLOB_BYTES=1048576 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000 CHUNK_SIZE_BYTES=1048576
import fs from 'node:fs';
const [port, dataDir, goodPath] = [process.argv[2], process.argv[3], process.argv[4]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };

class Client {
  constructor(tab) { this.cookie = ''; this.csrf = ''; this.tab = tab; }
  async req(method, path, { body, headers = {}, raw, tab } = {}) {
    const h = { ...headers };
    if (this.cookie) h.Cookie = this.cookie;
    if (this.csrf && method !== 'GET') h['X-CSRF-Token'] = this.csrf;
    const t = tab === undefined ? this.tab : tab;
    if (t) h['X-Tab-Id'] = t;
    let b = body;
    if (body && !raw && typeof body === 'object') { b = JSON.stringify(body); h['Content-Type'] = 'application/json'; }
    const r = await fetch(base + path, { method, headers: h, body: b });
    const buf = Buffer.from(await r.arrayBuffer());
    let json = null; try { json = JSON.parse(buf.toString()); } catch {}
    return { status: r.status, headers: r.headers, json, buf };
  }
  async start() {
    const r = await this.req('POST', '/sessions/anonymous', { tab: '' });
    this.cookie = r.headers.getSetCookie()[0].split(';')[0]; this.csrf = r.json.csrf_token;
  }
}
const sleep = ms => new Promise(r => setTimeout(r, ms));
async function poll(c, jid) {
  for (let i = 0; i < 200; i++) {
    const r = await c.req('GET', `/jobs/${jid}`);
    if (r.status !== 200) throw new Error('job lookup failed: ' + r.status + ' ' + JSON.stringify(r.json));
    const j = r.json;
    if (j.state === 'succeeded' || j.state === 'failed') return j;
    await sleep(30);
  }
  throw new Error('job did not finish');
}
async function upload(c, wsid, file) {
  const data = fs.readFileSync(file);
  const s = await c.req('POST', `/workspaces/${wsid}/uploads`, { body: { filename: 'test.rpf', size: data.length } });
  const cs = s.json.chunk_size;
  for (let i = 0; i < s.json.chunk_count; i++) await c.req('PUT', `/uploads/${s.json.upload_id}/chunks/${i}`, { body: data.subarray(i * cs, (i + 1) * cs), raw: true });
  const done = await c.req('POST', `/uploads/${s.json.upload_id}/complete`);
  return { job: await poll(c, done.json.job_id), data };
}
const put = (c, wsid, bytes, tab) => c.req('PUT', `/workspaces/${wsid}/blobs`, { body: bytes, raw: true, tab });
const op = (c, wsid, version, o, tab) => c.req('POST', `/workspaces/${wsid}/edit/ops`, { body: { version, ...o }, tab });
const find = async (c, vid, path) => {
  let id = '0';
  for (const part of path.split('/').filter(Boolean)) {
    const l = (await c.req('GET', `/versions/${vid}/entries?parent=${id}&limit=500`)).json;
    const e = l.entries.find(x => x.name === part);
    if (!e) return null; id = e.id;
  }
  return id;
};
const bytesOf = async (c, vid, path) => {
  const id = await find(c, vid, path);
  return id === null ? null : (await c.req('GET', `/versions/${vid}/entries/${id}/bytes`)).buf;
};

const A = new Client('tab-aaaaaaaa'), B = new Client('tab-bbbbbbbb');
await A.start();
const ws = (await A.req('POST', '/workspaces', { body: { name: 'm3' } })).json;
const up = await upload(A, ws.id, goodPath);
ok(up.job.state === 'succeeded', 'upload parsed');
const v1 = up.job.version_id;
const origF2 = await bytesOf(A, v1, '/d00000/f0002.bin');
const origF0 = await bytesOf(A, v1, '/d00000/f0000.bin');

console.log('# writer lock');
ok((await A.req('POST', `/workspaces/${ws.id}/edit/undo`, { tab: '' })).status === 400, 'missing tab id -> 400');
const w1 = await A.req('POST', `/workspaces/${ws.id}/writer`);
ok(w1.status === 200 && w1.json.writer === 'you', 'tab A claims writer');
// a second tab shares the same session cookie (same browser) but has a different tab id
B.cookie = A.cookie; B.csrf = A.csrf;
const w2 = await B.req('POST', `/workspaces/${ws.id}/writer`);
ok(w2.status === 409 && w2.json.error.code === 'writer_busy', 'tab B refused while A holds the lease');
ok((await B.req('GET', `/workspaces/${ws.id}/edit`)).json.writer === 'other', 'tab B sees read-only state');
const bop = await op(B, ws.id, v1, { op: 'mkdir', parent: '/', name: 'x' });
ok(bop.status === 409 && bop.json.error.code === 'writer_busy', 'tab B cannot edit');
ok((await B.req('DELETE', `/workspaces/${ws.id}/writer`)).status === 204 && (await B.req('GET', `/workspaces/${ws.id}/edit`)).json.writer === 'other', 'a non-holder cannot release');
ok((await A.req('DELETE', `/workspaces/${ws.id}/writer`)).status === 204, 'A releases');
ok((await B.req('POST', `/workspaces/${ws.id}/writer`)).status === 200, 'B claims after release');
ok((await B.req('DELETE', `/workspaces/${ws.id}/writer`)).status === 204 && (await A.req('POST', `/workspaces/${ws.id}/writer`)).status === 200, 'A claims again');

console.log('# operations, undo and redo');
let st = (await A.req('GET', `/workspaces/${ws.id}/edit`)).json;
ok(st.base_version === null && st.ops.length === 0 && !st.can_undo, 'fresh edit state');
ok((await A.req('POST', `/workspaces/${ws.id}/edit/undo`)).status === 409, 'undo with nothing -> 409');
ok((await A.req('POST', `/workspaces/${ws.id}/edit/save`)).status === 409, 'save with nothing -> 409');
const r1 = await op(A, ws.id, v1, { op: 'rename', path: '/d00000', new_name: 'docs' });
ok(r1.status === 200 && r1.json.cursor === 1 && r1.json.can_undo && !r1.json.can_redo && r1.json.base_version === v1, 'rename logged');
const r2 = await op(A, ws.id, v1, { op: 'mkdir', parent: '/docs', name: 'new' });
ok(r2.status === 200 && r2.json.cursor === 2, 'mkdir logged');
const content = Buffer.from('hello from the editor\n'.repeat(100));
const b1 = await put(A, ws.id, content);
ok(b1.status === 201 && b1.json.size === content.length, 'content uploaded');
const r3 = await op(A, ws.id, v1, { op: 'add_file', parent: '/docs/new', name: 'hello.txt', blob_id: b1.json.blob_id });
ok(r3.status === 200 && r3.json.ops[2].type === 'add_file' && r3.json.ops[2].size === content.length, 'add_file logged');
const repl = Buffer.alloc(5000, 7);
const b2 = await put(A, ws.id, repl);
ok((await op(A, ws.id, v1, { op: 'replace', path: '/docs/f0000.bin', blob_id: b2.json.blob_id })).status === 200, 'replace logged');
ok((await op(A, ws.id, v1, { op: 'delete', path: '/docs/f0001.bin' })).status === 200, 'delete logged');
st = (await A.req('GET', `/workspaces/${ws.id}/edit`)).json;
ok(st.ops.length === 5 && st.cursor === 5 && st.pending_content_bytes === content.length + repl.length, 'five operations, content bytes counted');
let u = (await A.req('POST', `/workspaces/${ws.id}/edit/undo`)).json;
ok(u.cursor === 4 && u.can_redo && u.ops[4].active === false, 'undo moves the cursor, keeps the entry');
u = (await A.req('POST', `/workspaces/${ws.id}/edit/redo`)).json;
ok(u.cursor === 5 && !u.can_redo, 'redo');
await A.req('POST', `/workspaces/${ws.id}/edit/undo`); await A.req('POST', `/workspaces/${ws.id}/edit/undo`);
const r4 = await op(A, ws.id, v1, { op: 'rename', path: '/docs/f0002.bin', new_name: 'renamed.bin' });
ok(r4.json.ops.length === 4 && r4.json.cursor === 4 && !r4.json.can_redo, 'a new operation drops the redo tail');
// re-apply the two dropped operations so the saved result is predictable
ok((await op(A, ws.id, v1, { op: 'replace', path: '/docs/f0000.bin', blob_id: b2.json.blob_id })).status === 200, 'replace again');
ok((await op(A, ws.id, v1, { op: 'delete', path: '/docs/f0001.bin' })).status === 200, 'delete again');

console.log('# hostile edits');
const bad = async (o, status, code, m) => { const r = await op(A, ws.id, v1, o); ok(r.status === status && r.json.error.code === code, `${m}: got ${r.status} ${r.json?.error?.code}`); };
await bad({ op: 'mkdir', parent: '/', name: '..' }, 422, 'NAME_INVALID', 'dot-dot name');
await bad({ op: 'mkdir', parent: '/', name: 'a/b' }, 422, 'NAME_INVALID', 'slash in name');
await bad({ op: 'mkdir', parent: '/', name: '' }, 422, 'NAME_INVALID', 'empty name');
await bad({ op: 'mkdir', parent: '/', name: 'x'.repeat(256) }, 422, 'NAME_INVALID', 'overlong name');
await bad({ op: 'mkdir', parent: '/', name: 'DOCS' }, 422, 'EDIT_NAME_EXISTS', 'case-insensitive duplicate');
await bad({ op: 'delete', path: '/' }, 422, 'EDIT_ROOT_PROTECTED', 'delete root');
await bad({ op: 'delete', path: '/nope' }, 422, 'EDIT_PATH_NOT_FOUND', 'missing path');
await bad({ op: 'mkdir', parent: '/docs/f0000.bin', name: 'x' }, 422, 'EDIT_NOT_A_FOLDER', 'mkdir under a file');
await bad({ op: 'replace', path: '/docs', blob_id: b2.json.blob_id }, 422, 'EDIT_TARGET_IS_FOLDER', 'replace a folder');
await bad({ op: 'explode' }, 400, 'invalid_request', 'unknown op');
await bad({ op: 'add_file', parent: '/', name: 'q', blob_id: 123456 }, 400, 'content_missing', 'unknown content');
await bad({ op: 'rename', path: '/docs' }, 400, 'invalid_request', 'missing field');
ok((await op(A, ws.id, 'deadbeef', { op: 'mkdir', parent: '/', name: 'zz' })).status === 404, 'unknown version -> 404');
const C = new Client('tab-cccccccc'); await C.start();
ok((await C.req('GET', `/workspaces/${ws.id}/edit`)).status === 404, 'another session cannot see the edit state');
ok((await op(C, ws.id, v1, { op: 'mkdir', parent: '/', name: 'zz' })).status === 404, 'another session cannot edit');
ok((await put(C, ws.id, Buffer.from('x'))).status === 404, 'another session cannot upload content');
st = (await A.req('GET', `/workspaces/${ws.id}/edit`)).json;
ok(st.ops.length === 6, 'rejected operations were not logged');

console.log('# dry run and save');
const dry = (await A.req('POST', `/workspaces/${ws.id}/edit/validate`)).json;
ok(dry.ok === true && dry.planned.entries === 6 && dry.planned.new_entries === 2, 'dry run plans 6 entries (2 new or replaced files)');
const sv = await A.req('POST', `/workspaces/${ws.id}/edit/save`);
ok(sv.status === 202, 'save accepted');
const midOp = await op(A, ws.id, v1, { op: 'mkdir', parent: '/', name: 'late' });
const job = await poll(A, sv.json.job_id);
if (midOp.status === 200) await A.req('POST', `/workspaces/${ws.id}/edit/discard`); // the save had already finished
ok(job.state === 'succeeded' && job.type === 'save' && job.report.ok === true && job.report.saved.entries === 6, 'save job succeeded with a clean report');
ok(midOp.status === 409 || midOp.status === 200, 'edit during save is refused or lost the race cleanly');
const v2 = job.version_id;
ok(v2 && v2 !== v1, 'new version id');
const vl = (await A.req('GET', `/workspaces/${ws.id}/versions`)).json.versions;
ok(vl.length === 2 && vl[1].origin === 'save' && vl[0].origin === 'upload' && vl[1].state === 'ready', 'two versions, newest from save');
ok((await bytesOf(A, v2, '/docs/new/hello.txt'))?.equals(content), 'added file bytes equal the upload');
ok((await bytesOf(A, v2, '/docs/f0000.bin'))?.equals(repl), 'replaced file bytes equal the upload');
ok((await find(A, v2, '/docs/f0001.bin')) === null && (await find(A, v2, '/d00000')) === null, 'deleted and renamed paths are gone');
ok((await bytesOf(A, v2, '/docs/renamed.bin'))?.equals(origF2), 'renamed file keeps its original bytes');
ok((await bytesOf(A, v1, '/d00000/f0000.bin'))?.equals(origF0), 'the old version is unchanged and still readable');
const dl = (await A.req('GET', `/versions/${v2}/download`)).buf;
ok(dl.length === vl[1].size && dl.readUInt32LE(0) === 0x52504637, 'new version downloads as an RPF7 file');
const vr = (await A.req('POST', `/versions/${v2}/validate`)).json;
ok(vr.ok === true && vr.errors === 0, 'standalone validate of the new version');
const vr1 = (await A.req('POST', `/versions/${v1}/validate`)).json;
ok(vr1.ok === true, 'standalone validate of the uploaded version');
st = (await A.req('GET', `/workspaces/${ws.id}/edit`)).json;
ok(st.ops.length === 0 && st.base_version === null && st.pending_content_bytes === 0, 'edit state cleared after save');
ok(!fs.readdirSync(`${dataDir}/${ws.id}`).some(f => f.startsWith('blob-') || f.startsWith('save-')), 'no content or temp files left behind');

console.log('# failed save leaves everything intact');
const b3 = await put(A, ws.id, Buffer.from('will vanish'));
await op(A, ws.id, v2, { op: 'add_file', parent: '/', name: 'ghost.txt', blob_id: b3.json.blob_id });
for (const f of fs.readdirSync(`${dataDir}/${ws.id}`)) if (f.startsWith('blob-')) fs.unlinkSync(`${dataDir}/${ws.id}/${f}`);
const sv2 = await A.req('POST', `/workspaces/${ws.id}/edit/save`);
const job2 = await poll(A, sv2.json.job_id);
ok(job2.state === 'failed' && job2.error.code === 'EDIT_CONTENT_MISSING' && job2.report.ok === false, 'save fails with a clear code when content is missing');
ok((await A.req('GET', `/workspaces/${ws.id}/versions`)).json.versions.length === 2, 'no version was published');
ok(!fs.readdirSync(`${dataDir}/${ws.id}`).some(f => f.startsWith('save-')), 'temp file removed');
st = (await A.req('GET', `/workspaces/${ws.id}/edit`)).json;
ok(st.saving === false && st.ops.length === 1, 'pending changes are kept and editing works again');
ok((await A.req('POST', `/workspaces/${ws.id}/edit/discard`)).json.ops.length === 0, 'discard clears the log');

console.log('# names table limit is reported, not hidden');
// 400 long distinct names do not fit in the 64 KiB names table; operations are accepted one by one, save is refused
// (kept small here: 60 names x 1,100 bytes is not allowed, so use the dry run with a smaller count of maximal names)
let accepted = 0;
for (let i = 0; i < 300; i++) {
  const r = await op(A, ws.id, v2, { op: 'mkdir', parent: '/', name: String(i).padStart(250, '0') });
  if (r.status === 200) accepted++; else break;
}
const dry2 = (await A.req('POST', `/workspaces/${ws.id}/edit/validate`)).json;
ok(accepted === 300 && dry2.ok === false && dry2.findings[0].code === 'RPF_NAMES_TOO_LARGE', 'dry run reports the names table limit');
const sv3 = await poll(A, (await A.req('POST', `/workspaces/${ws.id}/edit/save`)).json.job_id);
ok(sv3.state === 'failed' && sv3.error.code === 'RPF_NAMES_TOO_LARGE', 'save refuses with the same code');
await A.req('POST', `/workspaces/${ws.id}/edit/discard`);

console.log('# version cap and deletion');
ok((await A.req('DELETE', `/versions/${v1}`)).status === 204, 'delete an older version');
ok((await A.req('DELETE', `/versions/${v2}`)).status === 409, 'the only remaining version cannot be deleted');
let cur = v2, n = (await A.req('GET', `/workspaces/${ws.id}/versions`)).json.versions.length;
while (n < 10) {
  await op(A, ws.id, cur, { op: 'mkdir', parent: '/', name: `m${n}` });
  const j = await poll(A, (await A.req('POST', `/workspaces/${ws.id}/edit/save`)).json.job_id);
  if (j.state !== 'succeeded') { ok(false, 'save #' + n + ' failed ' + JSON.stringify(j.error)); break; }
  cur = j.version_id; n++;
}
ok(n === 10, 'ten versions saved');
await op(A, ws.id, cur, { op: 'mkdir', parent: '/', name: 'eleventh' });
const cap = await A.req('POST', `/workspaces/${ws.id}/edit/save`);
ok(cap.status === 409 && cap.json.error.code === 'version_cap', 'the 11th version is refused');
ok((await A.req('DELETE', `/versions/${cur}`)).status === 409, 'the version with pending changes cannot be deleted');
const first = (await A.req('GET', `/workspaces/${ws.id}/versions`)).json.versions[0].id;
ok((await A.req('DELETE', `/versions/${first}`)).status === 204, 'delete the oldest version');
const j11 = await poll(A, (await A.req('POST', `/workspaces/${ws.id}/edit/save`)).json.job_id);
ok(j11.state === 'succeeded', 'save works again after freeing a slot');
ok((await A.req('GET', `/versions/${first}/entries`)).status === 404, 'a deleted version is gone');

console.log('# quota');
await A.req('POST', `/workspaces/${ws.id}/edit/discard`);
const used = (await A.req('GET', '/me')).json;
const big = Buffer.alloc(1048576, 1);
let quotaHit = null;
for (let i = 0; i < 4; i++) { const r = await put(A, ws.id, big); if (r.status !== 201) { quotaHit = r; break; } }
ok(quotaHit && quotaHit.status === 413 && quotaHit.json.error.code === 'quota_exceeded', `content upload stops at the storage quota (used before: ${used.usage_bytes})`);
const tooBig = await put(A, ws.id, Buffer.alloc(1048577, 1));
ok(tooBig.status === 413 && tooBig.json.error.code === 'limit_exceeded', 'one oversized file is refused');
await A.req('POST', `/workspaces/${ws.id}/edit/discard`);

console.log('# delete data');
ok((await A.req('DELETE', `/workspaces/${ws.id}`)).status === 204, 'delete workspace');
ok(!fs.existsSync(`${dataDir}/${ws.id}`), 'workspace directory removed');
ok((await A.req('GET', `/workspaces/${ws.id}/edit`)).status === 404, 'edit state gone with the workspace');

console.log(`\n${fail === 0 ? 'M3 E2E PASS' : 'M3 E2E FAIL'}: ${pass} checks passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
