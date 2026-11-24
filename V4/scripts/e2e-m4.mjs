// M4 deployment-readiness checks: global storage cap, health endpoints, Secure cookie.
// Usage: node scripts/e2e-m4.mjs <port> <small.rpf>
// Server env: GLOBAL_QUOTA_BYTES=120000 COOKIE_SECURE=true (small.rpf is about 50 KB)
import fs from 'node:fs';
const [port, file] = [process.argv[2], process.argv[3]];
const base = `http://127.0.0.1:${port}/api/v1`;
let pass = 0, fail = 0;
const ok = (c, m) => { if (c) pass++; else { fail++; console.log('  FAIL:', m); } };
const data = fs.readFileSync(file);
async function session() {
  const r = await fetch(base + '/sessions/anonymous', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' });
  const j = await r.json();
  return { cookie: r.headers.getSetCookie()[0], csrf: j.csrf_token };
}
const call = async (s, m, p, body) => {
  const h = { Cookie: s.cookie.split(';')[0], 'X-CSRF-Token': s.csrf, 'X-Tab-Id': 'tab-m4-test-1' };
  if (body && !Buffer.isBuffer(body)) { h['Content-Type'] = 'application/json'; body = JSON.stringify(body); }
  const r = await fetch(base + p, { method: m, headers: h, body });
  let j = null; try { j = await r.json(); } catch {}
  return { status: r.status, json: j };
};
async function upload(s) {
  const w = (await call(s, 'POST', '/workspaces', { name: 'x' })).json;
  return call(s, 'POST', `/workspaces/${w.id}/uploads`, { filename: 'a.rpf', size: data.length });
}
ok((await fetch(base + '/health/live')).status === 200, 'health/live');
ok((await fetch(base + '/health/ready')).status === 200, 'health/ready (storage writable)');
const a = await session(), b = await session(), c = await session();
ok(/;\s*Secure/i.test(a.cookie) && /HttpOnly/i.test(a.cookie), 'cookie is Secure and HttpOnly when COOKIE_SECURE=true');
ok((await upload(a)).status === 201, 'session A can start an upload');
ok((await upload(b)).status === 201, 'session B can start an upload');
const full = await upload(c);
ok(full.status === 503 && full.json.error.code === 'server_full', `session C is refused when the server is full (got ${full.status})`);
console.log(`\n${fail === 0 ? 'M4 E2E PASS' : 'M4 E2E FAIL'}: ${pass} checks passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
