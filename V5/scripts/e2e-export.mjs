// M5a export checks: GET /versions/{id}/export returns a valid ZIP. Usage: node scripts/e2e-export.mjs <port> <compressed.rpf>
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
const w = (await req('POST', '/workspaces', { body: { name: 'export' } })).json;
const st = (await req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'c.rpf', size: data.length } })).json;
const cs = st.chunk_size;
for (let i = 0; i < st.chunk_count; i++) await req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: data.subarray(i * cs, (i + 1) * cs), raw: true });
const done = await req('POST', `/uploads/${st.upload_id}/complete`);
let job = done.json;
for (let i = 0; i < 100; i++) { job = (await req('GET', `/jobs/${done.json.job_id}`)).json; if (job.state === 'succeeded' || job.state === 'failed') break; await new Promise(r => setTimeout(r, 50)); }
ok(job.state === 'succeeded', `parse job succeeded (got ${job.state})`);
const vid = done.json.version_id;
const FIXED = 'hello hello hello, RPF editor! hello hello\n', STORED = 'line one\nline two\nline three\n';
const r = await req('GET', `/versions/${vid}/export`);
ok(r.status === 200 && r.headers.get('content-type') === 'application/zip', `export returns a zip (status ${r.status})`);
ok(/filename="[^"]+\.zip"/.test(r.headers.get('content-disposition') || ''), 'download has a .zip file name');
const zp = process.env.TMPDIR ? process.env.TMPDIR + '/export-check.zip' : '/tmp/export-check.zip';
fs.writeFileSync(zp, r.buf);
import { execFileSync } from 'node:child_process';
const py = `import zipfile,sys,json\nz=zipfile.ZipFile(sys.argv[1]);bad=z.testzip()\nprint(json.dumps({"bad":bad,"files":{n:z.read(n).decode("latin1") for n in z.namelist()}}))`;
const out = JSON.parse(execFileSync('python3', ['-I', '-c', py, zp], { maxBuffer: 1 << 28 }).toString());
ok(out.bad === null, 'every CRC in the zip is valid (zipfile.testzip)');
ok(out.files['fixed.txt'] === FIXED, 'fixed.txt is inflated to the plain text');
ok(out.files['stored.txt'] === STORED, 'stored.txt is inflated to the plain text');
ok(Object.keys(out.files).includes('bomb.bin') && Object.keys(out.files).includes('_export_notes.txt'), 'bomb.bin is present and a notes file explains the exception');
ok(/bomb\.bin: written as stored bytes/.test(out.files['_export_notes.txt'] || ''), 'notes name bomb.bin and why');
const bad = await fetch(`${base}/versions/nope/export`, { headers: { Cookie: cookie } });
ok(bad.status === 404, 'unknown version gives 404');
const anon = await fetch(`${base}/versions/${vid}/export`);
ok(anon.status === 401 || anon.status === 403 || anon.status === 404, `no session gives ${anon.status}`);
console.log(`${fail === 0 ? 'PASS' : 'FAIL'} export: ${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
