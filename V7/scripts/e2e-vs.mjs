// visualsettings.dat editor checks (server routes + real save round trip). Usage: node scripts/e2e-vs.mjs <port> <vs.rpf>
// Make the archive: cargo run --release -p rpf-core --example gen -- vs.rpf vs   (synthetic file, not a game file)
import fs from 'node:fs';
const [port, file] = [process.argv[2], process.argv[3]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };
const data = fs.readFileSync(file);
let cookie = '', csrf = '';
async function req(method, path, { body, raw } = {}) {
  const h = { 'X-Tab-Id': 'tab-vs000001' };
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
const w = (await req('POST', '/workspaces', { body: { name: 'vs' } })).json;
const st = (await req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'p.rpf', size: data.length } })).json;
for (let i = 0; i < st.chunk_count; i++) await req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: data.subarray(i * st.chunk_size, (i + 1) * st.chunk_size), raw: true });
const done = await req('POST', `/uploads/${st.upload_id}/complete`);
const job = await poll(done.json.job_id);
ok(job.state === 'succeeded', 'parse job succeeded');
const v1 = done.json.version_id;
const ORIG = "# visual settings\r\nmisc.DamageOverlay.Duration 750.0f\r\n  car.headlight.color.red 255 # note\r\ncar.headlight.color.green 200\r\ncar.headlight.color.blue 100\r\nsky.sun.r 0.5\r\nsky.sun.g 0.25\r\nsky.sun.b 1.0\r\nmisc.DamageOverlay.Duration 750.0f\r\nbloom.threshold 0.8\r\n";
const EXPECT = ORIG.replace(/750\.0f/g, '900.5f').replace('red 255 #', 'red 128 #').replace('sky.sun.g 0.25', 'sky.sun.g 0.75');
const entry = (vid, name) => req('GET', `/versions/${vid}/entries`).then(r => r.json.entries.find(e => e.name === name));
const NAMES = ['visualsettings.dat', 'zz_visualsettings.dat'];
const lines = 'misc.DamageOverlay.Duration\t900.5\ncar.headlight.color.red\t128\nsky.sun.g\t0.75\n';
let cur = done.json.version_id;
const ents = {};
for (const name of NAMES) {
  const e = await entry(cur, name);
  ents[name] = e;
  ok(e && e.kind === 'file', name + ' is a file entry');
  ok(name.startsWith('zz') ? e.compressed === true : e.compressed === false, name + ' compressed flag as expected');
  let r = await req('GET', `/versions/${cur}/entries/${e.id}/vs`);
  ok(r.status === 200, name + ' read ok');
  const keys = r.json.entries.map(x => x.key);
  ok(keys.length === 8 && keys[0] === 'misc.DamageOverlay.Duration', name + ' 8 distinct settings');
  const d = r.json.entries.find(x => x.key === 'misc.DamageOverlay.Duration');
  ok(d.repeats === 2 && d.value === '750.0f' && d.line === 2, name + ' repeated key reported');
  ok(r.json.colours.length === 2, name + ' two colour triples');
  const c = r.json.colours.find(x => x.base === 'car.headlight.color');
  ok(c && c.scale === 255 && c.keys[0] === 'car.headlight.color.red', name + ' colour scale 255 detected');
  ok(r.json.colours.find(x => x.base === 'sky.sun').scale === 1, name + ' colour scale 1 detected');
  r = await req('POST', `/versions/${cur}/entries/${e.id}/vs/edit`, { body: 'nope.key\t1\n', raw: true });
  ok(r.status === 422 && r.json.error.code === 'unknown_setting', name + ' unknown key rejected');
  r = await req('POST', `/versions/${cur}/entries/${e.id}/vs/edit`, { body: 'bloom.threshold\t1 2\n', raw: true });
  ok(r.status === 400, name + ' value with a space rejected');
  r = await req('POST', `/versions/${cur}/entries/${e.id}/vs/edit`, { body: 'garbage', raw: true });
  ok(r.status === 400, name + ' malformed line rejected');
  r = await req('POST', `/versions/${cur}/entries/${e.id}/vs/edit`, { body: lines, raw: true });
  ok(r.status === 200 && r.buf.toString('utf8') === EXPECT, name + ' edit output is exactly the original with only the 4 lines changed, f suffix kept, CRLF and comment kept');
  const b = await req('PUT', `/workspaces/${w.id}/blobs`, { body: r.buf, raw: true });
  ok(b.status === 201, name + ' content uploaded');
  const o = await req('POST', `/workspaces/${w.id}/edit/ops`, { body: { version: cur, op: 'replace', path: '/' + name, blob_id: b.json.blob_id } });
  ok(o.status === 200, name + ' replace queued (' + o.status + ')');
  const sv = await req('POST', `/workspaces/${w.id}/edit/save`);
  const sj = await poll(sv.json.job_id);
  ok(sj.state === 'succeeded' && sj.report.ok === true, name + ' save clean (' + JSON.stringify(sj.error || sj.report).slice(0, 160) + ')');
  cur = sj.version_id;
}
// reopen: both files carry the edit; the file that was not saved second still reads correctly
for (const name of NAMES) {
  const e = await entry(cur, name);
  const r = await req('GET', `/versions/${cur}/entries/${e.id}/vs`);
  const get = (k) => r.json.entries.find(x => x.key === k).value;
  ok(get('misc.DamageOverlay.Duration') === '900.5f' && get('car.headlight.color.red') === '128' && get('sky.sun.g') === '0.75' && get('bloom.threshold') === '0.8', name + ' saved values read back');
  const raw = await req('GET', `/versions/${cur}/entries/${e.id}/bytes?basis=uncompressed`, {});
  ok(raw.buf.toString('utf8') === EXPECT, name + ' saved bytes equal the expected text');
}
const vv = await req('POST', `/versions/${cur}/validate`);
ok(vv.status === 200 && vv.json.ok !== false, 'saved archive validates');
console.log(`${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
