// Particle editor checks (server routes + real save round trip). Usage: node scripts/e2e-ypt.mjs <port> <ypt.rpf>
// Make the archive: cargo run --release -p rpf-core --example gen -- ypt.rpf ypt   (synthetic dictionary, not a game file)
import fs from 'node:fs';
const [port, file] = [process.argv[2], process.argv[3]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };
const data = fs.readFileSync(file);
let cookie = '', csrf = '';
async function req(method, path, { body, raw } = {}) {
  const h = { 'X-Tab-Id': 'tab-ypt00001' };
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
const w = (await req('POST', '/workspaces', { body: { name: 'ypt' } })).json;
const st = (await req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'p.rpf', size: data.length } })).json;
for (let i = 0; i < st.chunk_count; i++) await req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: data.subarray(i * st.chunk_size, (i + 1) * st.chunk_size), raw: true });
const done = await req('POST', `/uploads/${st.upload_id}/complete`);
const job = await poll(done.json.job_id);
ok(job.state === 'succeeded', 'parse job succeeded');
const v1 = done.json.version_id;
const entry = (vid) => req('GET', `/versions/${vid}/entries`).then(r => r.json.entries.find(e => e.name === 'test.ypt'));
const e1 = await entry(v1);
ok(e1 && e1.kind === 'resource', 'test.ypt is a resource entry');

let r = await req('GET', `/versions/${v1}/entries/${e1.id}/ypt`);
ok(r.status === 200 && r.json.groups.length === 1, 'ypt read returns one group');
const g = r.json.groups[0];
ok(g.kind === 'effect' && g.name === 'eff1' && g.properties[0].name === 'ptxEffectRule:m_colourTintMaxKFP', 'group and property names decoded');
const fr = g.properties[0].frames;
ok(fr.length === 2 && fr[1].value[0] === 1.5, 'keyframe values decoded');

// invalid requests
r = await req('POST', `/versions/${v1}/entries/${e1.id}/ypt/edit`, { body: `${fr[1].offset + 1} 0 0 0 0`, raw: true });
ok(r.status === 422 && r.json.error.code === 'invalid_change', 'offset that is not a keyframe value is rejected');
r = await req('POST', `/versions/${v1}/entries/${e1.id}/ypt/edit`, { body: 'abc', raw: true });
ok(r.status === 400, 'malformed change line is rejected');
const root = (await req('GET', `/versions/${v1}/entries`)).json.entries;
r = await req('GET', `/versions/${v1}/entries/0/ypt`);
ok(r.status === 422, 'folder is not a resource');

// edit -> blob -> replace op -> save -> reopen
r = await req('POST', `/versions/${v1}/entries/${e1.id}/ypt/edit`, { body: `${fr[1].offset} 0.1 0.2 0.3 0.4\n`, raw: true });
ok(r.status === 200 && r.buf.length > 16 && r.buf.readUInt32LE(0) === 0x37435352, 'edit returns an RSC7 resource');
const b = await req('PUT', `/workspaces/${w.id}/blobs`, { body: r.buf, raw: true });
ok(b.status === 201, 'resource uploaded as pending content');
const o = await req('POST', `/workspaces/${w.id}/edit/ops`, { body: { version: v1, op: 'replace', path: '/test.ypt', blob_id: b.json.blob_id } });
ok(o.status === 200, 'replace op queued (' + o.status + ' ' + JSON.stringify(o.json).slice(0, 120) + ')');
const sv = await req('POST', `/workspaces/${w.id}/edit/save`);
const sj = await poll(sv.json.job_id);
ok(sj.state === 'succeeded' && sj.report.ok === true, 'save succeeded with a clean report (' + JSON.stringify(sj.error || sj.report).slice(0, 200) + ')');
const v2 = sj.version_id;
const e2 = await entry(v2);
r = await req('GET', `/versions/${v2}/entries/${e2.id}/ypt`);
ok(r.status === 200 && JSON.stringify(r.json.groups[0].properties[0].frames[1].value) === JSON.stringify([0.1, 0.2, 0.3, 0.4]) || Math.abs(r.json.groups[0].properties[0].frames[1].value[0] - 0.1) < 1e-6, 'saved archive shows the new values');
ok(Math.abs(r.json.groups[0].properties[0].frames[0].value[0] - 0.5) < 1e-6, 'untouched keyframe unchanged');
const vv = await req('POST', `/versions/${v2}/validate`);
ok(vv.status === 200 && vv.json.ok !== false, 'saved archive validates');
console.log(`${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
