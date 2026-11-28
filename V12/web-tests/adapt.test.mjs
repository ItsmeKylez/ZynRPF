// Runs under plain Node (type stripping): `node --test web-tests/`
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { KeyedQueue } from "../src/web/queue.ts";
import { EXPORT_DIALOG_TITLE, WEB_DOWNLOAD_HANDLE, WEB_PICK_PREFIX, fileIdFromHandle, toBelgeOzet, toDosyaIcerigi, toKayitSonucu, toNode, toPackage, toSettings, webFileHandle } from "../src/web/adapt.ts";

const wireDoc = {
  docId: 3,
  name: "mod.rpf",
  encryption: "OPEN",
  diskSize: 1536,
  estimatedSize: 1400,
  dirty: false,
  undoDepth: 0,
  redoDepth: 0,
  revision: 7,
  package: { name: "My Mod", version: "1.0", author: "me", description: "d", targets: [{ source: "a.ytd", archives: ["x.rpf"], path: "p", delete: false }] },
  packageError: null,
  missingSources: ["m.ytd"],
  tree: [
    { name: "content", path: "content", kind: "directory", size: 6, storedSize: 6, modified: false, resource: false,
      children: [{ name: "readme.txt", path: "content/readme.txt", kind: "file", size: 5, storedSize: 8, modified: true, resource: true, children: [] }] },
  ],
};

test("kinds map to the desktop vocabulary", () => {
  assert.equal(toNode({ ...wireDoc.tree[0] }).tur, "dizin");
  assert.equal(toNode({ ...wireDoc.tree[0], kind: "archive" }).tur, "arsiv");
  assert.equal(toNode({ ...wireDoc.tree[0], kind: "file" }).tur, "dosya");
});

test("an unknown node kind is an error, not a silent default", () => {
  assert.throws(() => toNode({ ...wireDoc.tree[0], kind: "symlink" }), /Unknown node kind/);
});

test("document summary maps every field including nested tree", () => {
  const o = toBelgeOzet(wireDoc, "abc123");
  assert.deepEqual(
    { id: o.id, yol: o.yol, ad: o.ad, sifreleme: o.sifreleme, disk: o.disk_boyutu, tahmini: o.tahmini_boyut, kirli: o.kirli, geri: o.geri, ileri: o.ileri, durum: o.durum_no, hata: o.paket_hatasi, eksik: o.eksik_kaynak },
    { id: 3, yol: "web-file:abc123", ad: "mod.rpf", sifreleme: "OPEN", disk: 1536, tahmini: 1400, kirli: false, geri: 0, ileri: 0, durum: 7, hata: null, eksik: ["m.ytd"] },
  );
  const child = o.agac[0].cocuklar[0];
  assert.deepEqual(child, { ad: "readme.txt", yol: "content/readme.txt", tur: "dosya", boyut: 5, kayitli: 8, degisti: true, rsc: true, cocuklar: [] });
});

test("package and targets map; absent package stays null", () => {
  assert.deepEqual(toPackage(wireDoc.package), { ad: "My Mod", surum: "1.0", yazar: "me", aciklama: "d", hedefler: [{ kaynak: "a.ytd", arsivler: ["x.rpf"], yol: "p", sil: false }] });
  assert.equal(toBelgeOzet({ ...wireDoc, package: null }).paket, null);
});

test("no fileId means no handle (unsaved-style null)", () => {
  assert.equal(toBelgeOzet(wireDoc).yol, null);
});

test("web-file handles round-trip and reject other paths", () => {
  assert.equal(fileIdFromHandle(webFileHandle("f00d")), "f00d");
  assert.equal(fileIdFromHandle("C:\\mods\\a.rpf"), null);
  assert.equal(fileIdFromHandle("web-file:"), null);
  assert.equal(fileIdFromHandle("/etc/passwd"), null);
});

test("entry kinds map to the desktop vocabulary", () => {
  const base = { size: 4, text: null, hexDump: null, rscVersion: null };
  const kinds = { meta: "meta", texture: "ytd", model: "model", resource: "kaynak", setting: "ayar", text: "metin", binary: "ikili" };
  for (const [wire, desktop] of Object.entries(kinds)) assert.equal(toDosyaIcerigi({ ...base, kind: wire }).tur, desktop, wire);
  assert.deepEqual(toDosyaIcerigi({ kind: "text", size: 5, text: "hello", hexDump: null, rscVersion: 12 }), { tur: "metin", boyut: 5, metin: "hello", hex: null, rsc_surum: 12 });
  assert.throws(() => toDosyaIcerigi({ ...base, kind: "weird" }), /Unknown entry kind/);
});

test("save result maps to the desktop KayitSonucu", () => {
  const r = toKayitSonucu({ summary: wireDoc, oldSize: 10, newSize: 12, backupId: "000003" }, "f1");
  assert.equal(r.eski_boyut, 10);
  assert.equal(r.yeni_boyut, 12);
  assert.equal(r.yedek, "000003");
  assert.equal(r.ozet.yol, "web-file:f1");
  assert.equal(toKayitSonucu({ summary: wireDoc, oldSize: 0, newSize: 1, backupId: null }).yedek, null);
});

test("dialog handles cannot be confused with web-file handles or paths", () => {
  assert.equal(fileIdFromHandle(WEB_PICK_PREFIX + "1"), null);
  assert.equal(fileIdFromHandle(WEB_DOWNLOAD_HANDLE), null);
});

test("export dialog title is still the literal used by the desktop Belge view", () => {
  // The dialog shim recognises "export to folder" by this title. If M7 translates it, update adapt.ts too.
  assert.ok(readFileSync(new URL("../src/Belge.tsx", import.meta.url), "utf8").includes(`title: "${EXPORT_DIALOG_TITLE}"`));
});

test("KeyedQueue runs tasks of one key in order, keys in parallel, and survives failures", async () => {
  const q = new KeyedQueue();
  const log = [];
  const gate = (ms, name, fail) => () => new Promise((res, rej) => setTimeout(() => { log.push(name); fail ? rej(new Error(name)) : res(name); }, ms));
  const a1 = q.run("a", gate(30, "a1"));
  const a2 = q.run("a", gate(1, "a2-fails", true));
  const a3 = q.run("a", gate(1, "a3"));
  const b1 = q.run("b", gate(5, "b1"));
  await assert.rejects(a2, /a2-fails/);
  assert.deepEqual(await Promise.all([a1, a3, b1]), ["a1", "a3", "b1"]);
  assert.deepEqual(log, ["b1", "a1", "a2-fails", "a3"]);
});

test("settings map to the desktop Girdi / RenkUclusu shapes", () => {
  const r = toSettings({ entries: [{ key: "a.b", value: "1.5f", line: 3, repeat: 2 }], colors: [{ base: "c", keys: ["c.r", "c.g", "c.b"], scale: 255 }] });
  assert.deepEqual(r, { girdiler: [{ anahtar: "a.b", deger: "1.5f", satir: 3, tekrar: 2 }], renkler: [{ taban: "c", anahtarlar: ["c.r", "c.g", "c.b"], olcek: 255 }] });
});
