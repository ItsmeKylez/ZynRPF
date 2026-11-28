"use strict";
const $ = (id) => document.getElementById(id);
const H = { "X-Requested-With": "zyn" };
let current = null; // { docId, fileId }

function status(msg, isError) {
  const el = $("status");
  el.textContent = msg;
  el.className = isError ? "error" : "";
}

async function api(path, opts) {
  const res = await fetch("/api/v1" + path, Object.assign({ credentials: "same-origin" }, opts));
  if (!res.ok) {
    let msg = res.status + " " + res.statusText;
    try { const j = await res.json(); if (j.error) msg = j.error.code + ": " + j.error.message; } catch (_) { /* not JSON */ }
    throw new Error(msg);
  }
  return res;
}

function renderNode(node, docId) {
  const li = document.createElement("li");
  if (node.kind === "file") {
    const a = document.createElement("a");
    a.textContent = node.name; // textContent only: archive entry names are untrusted
    a.href = "/api/v1/documents/" + docId + "/entry?path=" + encodeURIComponent(node.path);
    li.appendChild(a);
  } else {
    li.appendChild(document.createTextNode(node.name + (node.kind === "archive" ? " (archive)" : "/")));
  }
  const meta = document.createElement("span");
  meta.className = "meta";
  meta.textContent = " " + node.size + " B";
  li.appendChild(meta);
  if (node.children && node.children.length) {
    const ul = document.createElement("ul");
    ul.className = "tree";
    node.children.forEach((c) => ul.appendChild(renderNode(c, docId)));
    li.appendChild(ul);
  }
  return li;
}

function show(summary, fileId) {
  current = { docId: summary.docId, fileId };
  $("summary").textContent = summary.name + " — encryption: " + summary.encryption + ", " + summary.diskSize + " bytes on disk";
  const tree = $("tree");
  tree.replaceChildren();
  summary.tree.forEach((n) => tree.appendChild(renderNode(n, summary.docId)));
  $("export").href = "/api/v1/documents/" + summary.docId + "/export";
  $("original").href = "/api/v1/files/" + fileId + "/download";
  $("actions").hidden = false;
}

$("open").addEventListener("click", async () => {
  const f = $("file").files[0];
  if (!f) { status("Choose an .rpf file first.", true); return; }
  try {
    status("Uploading…");
    const fd = new FormData();
    fd.append("file", f, f.name);
    const up = await (await api("/files", { method: "POST", headers: H, body: fd })).json();
    status("Opening…");
    const sum = await (await api("/documents", {
      method: "POST",
      headers: Object.assign({ "Content-Type": "application/json" }, H),
      body: JSON.stringify({ fileId: up[0].fileId }),
    })).json();
    show(sum, up[0].fileId);
    status("Opened " + sum.name + ".");
  } catch (e) {
    status(e.message, true);
  }
});

$("close").addEventListener("click", async () => {
  if (!current) return;
  try {
    await api("/documents/" + current.docId, { method: "DELETE", headers: H });
    current = null;
    $("tree").replaceChildren();
    $("summary").textContent = "";
    $("actions").hidden = true;
    status("Document closed.");
  } catch (e) {
    status(e.message, true);
  }
});
