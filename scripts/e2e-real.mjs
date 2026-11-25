// REAL game file test (not part of run-all-checks.sh; needs the user's Zyn.rpf).
// Usage: node scripts/e2e-real.mjs <port> <Zyn.rpf>
// Server env: SESSION_QUOTA_BYTES=100000000 RATE_READS_PER_MIN=5000 RATE_WRITES_PER_MIN=5000
// Flow: open Zyn.rpf -> open nested ptfx_hi.rpf -> read core.ypt -> bulk change (colour/light values)
//       -> queue -> write back into ptfx_hi.rpf -> write back into Zyn.rpf -> save -> reopen -> verify -> validate.
import fs from 'node:fs';
const [port, file] = [process.argv[2], process.argv[3]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) { pass++; console.log('  ok  ', m); } else { fail++; console.log('  FAIL', m); } };
const data = fs.readFileSync(file);
let cookie = '', csrf = '';
async function req(method, path, { body, raw, range } = {}) {
  const h = { 'X-Tab-Id': 'tab-real0001' };
  if (range) h.Range = range;
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
const poll = async jid => { for (let i = 0; i < 600; i++) { const j = (await req('GET', `/jobs/${jid}`)).json; if (j.state === 'succeeded' || j.state === 'failed') return j; await sleep(50); } throw new Error('job timeout'); };
const find = async (vid, path) => { let id = '0'; for (const p of path.split('/').filter(Boolean)) { const l = (await req('GET', `/versions/${vid}/entries?parent=${id}&limit=500`)).json; const e = l.entries.find(x => x.name === p); if (!e) return null; id = e.id; } return id; };
const walk = async (vid, id = '0', prefix = '') => { const l = (await req('GET', `/versions/${vid}/entries?parent=${id}&limit=500`)).json.entries; let out = []; for (const e of l) { const p = prefix + '/' + e.name; out.push({ ...e, path: p }); if (e.kind === 'directory' || e.kind === 'dir' || e.kind === 'folder') out = out.concat(await walk(vid, e.id, p)); } return out; };
const sv = j => j.version || j.version_id;

const s0 = await req('POST', '/sessions/anonymous'); cookie = s0.headers.getSetCookie()[0].split(';')[0]; csrf = s0.json.csrf_token;
const w = (await req('POST', '/workspaces', { body: { name: 'real' } })).json;
const st = (await req('POST', `/workspaces/${w.id}/uploads`, { body: { filename: 'Zyn.rpf', size: data.length } })).json;
for (let i = 0; i < st.chunk_count; i++) await req('PUT', `/uploads/${st.upload_id}/chunks/${i}`, { body: data.subarray(i * st.chunk_size, (i + 1) * st.chunk_size), raw: true });
const done = await req('POST', `/uploads/${st.upload_id}/complete`);
const job = await poll(done.json.job_id);
ok(job.state === 'succeeded', 'real Zyn.rpf parsed by the server');
const v1 = done.json.version_id;

const all = await walk(v1);
const ptfx = all.find(e => e.name === 'ptfx_hi.rpf');
ok(!!ptfx, `found ptfx_hi.rpf at ${ptfx && ptfx.path}`);
const o = await req('POST', `/versions/${v1}/entries/${ptfx.id}/open`);
ok(o.status === 201, `ptfx_hi.rpf opened as nested archive (${o.status})`);
const nv = o.json.version_id;
const yid = await find(nv, '/core.ypt');
const yinfo = (await req('GET', `/versions/${nv}/entries/${yid}`)).json;
ok(yinfo.kind === 'resource', `core.ypt is a resource entry, stored size ${yinfo.stored_size ?? yinfo.size}`);
const origHead = (await req('GET', `/versions/${nv}/entries/${yid}/bytes`, { range: 'bytes=0-15' })).buf;
const origStored = { length: yinfo.stored_size ?? yinfo.size, readUInt32LE: o => origHead.readUInt32LE(o) };
console.log('  stored core.ypt bytes:', origStored.length, 'magic', origHead.readUInt32LE(0).toString(16));

let t0 = Date.now();
const rd = await req('GET', `/versions/${nv}/entries/${yid}/ypt`);
ok(rd.status === 200, `real core.ypt decoded and read (${rd.status} ${rd.status !== 200 ? rd.buf.toString().slice(0, 200) : ''}) in ${Date.now() - t0} ms`);
const groups = rd.json.groups;
let nProps = 0, nFrames = 0, bad = 0;
const kinds = {};
for (const g of groups) { kinds[g.kind] = (kinds[g.kind] || 0) + 1; for (const p of g.properties) { nProps++; for (const f of p.frames) { nFrames++; if (![...f.value, ...f.time].every(Number.isFinite)) bad++; } } }
console.log('  groups', groups.length, JSON.stringify(kinds), 'properties', nProps, 'keyframes', nFrames);
ok(groups.length > 0 && nFrames > 0, 'real ypt has groups and keyframes');
ok(bad === 0, 'all keyframe numbers are finite');
const named = groups.filter(g => g.name && g.name.length > 0).length;
ok(named > groups.length * 0.5, `group names look plausible (${named}/${groups.length} non-empty); e.g. ${groups.slice(0, 3).map(g => g.kind + ':' + g.name).join(', ')}`);
const known = groups.flatMap(g => g.properties).filter(p => !p.name.startsWith('hash_')).length;
ok(known > nProps * 0.5, `property hashes resolve to names (${known}/${nProps})`);
const times = groups.flatMap(g => g.properties.flatMap(p => p.frames.map(f => f.time[0])));
ok(times.every(t => t >= -0.001 && t <= 1.001), 'keyframe time (x) values are within 0..1 as the layout predicts');

// bulk change: halve rgb of every Colour/Light rgb max/min property (real values), capped.
const changes = new Map();
for (const g of groups) for (const p of g.properties) if (/rgba?M(ax|in)KFP$/.test(p.name)) for (const f of p.frames) { if (changes.size < 20000) changes.set(f.offset, [f.value[0] * 0.5, f.value[1] * 0.5, f.value[2] * 0.5, f.value[3]]); }
console.log('  bulk change touches', changes.size, 'keyframe values');
ok(changes.size > 0, 'bulk change found colour keyframes in the real file');
const lines = [...changes].map(([o, v]) => `${o} ${v.join(' ')}`).join('\n');
t0 = Date.now();
const ed = await req('POST', `/versions/${nv}/entries/${yid}/ypt/edit`, { body: lines, raw: true });
ok(ed.status === 200 && ed.buf.readUInt32LE(0) === 0x37435352, `edit returned a resource (${ed.status}) in ${Date.now() - t0} ms`);
console.log(`  edited resource ${ed.buf.length} bytes vs original stored ${origStored.length} (ratio ${(ed.buf.length / origStored.length).toFixed(3)})`);
ok(ed.buf.length < origStored.length * 1.5, 'edited resource is real-compressed, not inflated to the decoded size');
ok(ed.buf.readUInt32LE(8) === origStored.readUInt32LE(8) && ed.buf.readUInt32LE(12) === origStored.readUInt32LE(12), 'RSC7 page flags unchanged');

await req('POST', `/workspaces/${w.id}/writer`);
const b = await req('PUT', `/workspaces/${w.id}/blobs`, { body: ed.buf, raw: true });
ok(b.status === 201, `edited core.ypt queued as pending content (${b.status})`);
const op1 = await req('POST', `/workspaces/${w.id}/edit/ops`, { body: { version: nv, op: 'replace', path: '/core.ypt', blob_id: b.json.blob_id } });
ok(op1.status === 200, `replace op on ptfx_hi.rpf (${op1.status} ${op1.status !== 200 ? op1.buf.toString().slice(0, 200) : ''})`);
const s1 = await poll((await req('POST', `/workspaces/${w.id}/edit/save`)).json.job_id);
ok(s1.state === 'succeeded', `ptfx_hi.rpf saved (${JSON.stringify(s1.report || s1.error).slice(0, 150)})`);
const nv2 = sv(s1);
const fb = await req('POST', `/workspaces/${w.id}/blobs/from-version`, { body: { version: nv2 } });
ok(fb.status === 201, `edited ptfx_hi.rpf copied as pending content (${fb.status})`);
const op2 = await req('POST', `/workspaces/${w.id}/edit/ops`, { body: { version: v1, op: 'replace', path: ptfx.path, blob_id: fb.json.blob_id } });
ok(op2.status === 200, `replace op on Zyn.rpf (${op2.status} ${op2.status !== 200 ? op2.buf.toString().slice(0, 200) : ''})`);
const s2 = await poll((await req('POST', `/workspaces/${w.id}/edit/save`)).json.job_id);
ok(s2.state === 'succeeded' && s2.report && s2.report.ok === true, `Zyn.rpf saved with a clean report (${JSON.stringify(s2.report || s2.error).slice(0, 200)})`);
const v2 = sv(s2);
const dl = (await req('GET', `/versions/${v2}/download`)).buf;
console.log('  saved Zyn.rpf size', dl.length, 'original', data.length);
fs.writeFileSync(process.env.OUT_RPF || '/tmp/zyn_edited.rpf', dl);

const vv = await req('POST', `/versions/${v2}/validate`);
ok(vv.status === 200 && vv.json.ok !== false, `edited Zyn.rpf validates (${JSON.stringify(vv.json).slice(0, 200)})`);
const all2 = await walk(v2);
const orig = Object.fromEntries(all.filter(e => e.name !== 'ptfx_hi.rpf').map(e => [e.path, e.size]));
ok(all2.filter(e => e.name !== 'ptfx_hi.rpf').every(e => orig[e.path] === e.size) && all2.length === all.length, 'every other entry keeps its path and size');
const ptfx2 = all2.find(e => e.name === 'ptfx_hi.rpf');
const o2 = await req('POST', `/versions/${v2}/entries/${ptfx2.id}/open`);
ok(o2.status === 201, 'edited ptfx_hi.rpf reopens from the saved Zyn.rpf');
const yid2 = await find(o2.json.version_id, '/core.ypt');
const rd2 = await req('GET', `/versions/${o2.json.version_id}/entries/${yid2}/ypt`);
ok(rd2.status === 200, 'edited core.ypt reads back');
let wrong = 0, changedSeen = 0, untouchedWrong = 0;
const after = new Map();
for (const g of rd2.json.groups) for (const p of g.properties) for (const f of p.frames) after.set(f.offset, f.value);
for (const g of groups) for (const p of g.properties) for (const f of p.frames) {
  const a = after.get(f.offset); const want = changes.get(f.offset) || f.value;
  if (!a) { wrong++; continue; }
  if (changes.has(f.offset)) changedSeen++;
  if (!a.every((x, i) => Math.abs(x - want[i]) < 1e-6)) { changes.has(f.offset) ? wrong++ : untouchedWrong++; }
}
ok(wrong === 0 && changedSeen === changes.size, `all ${changedSeen} changed keyframes hold the new values`);
ok(untouchedWrong === 0 && after.size === nFrames, `all other ${nFrames - changes.size} keyframes unchanged, same count`);
console.log(`${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
