"use strict";

const app = document.getElementById("app");
const topStats = document.getElementById("topbar-stats");

let config = null;
let scanOptions = [];
let selectedOption = 0;
let customPath = "";
let minSize = 1;
let maxDepth = 0;
let results = null;
const selected = new Set();

// ---------- helpers ----------
function fmtBytes(n) {
  if (n === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(n) / Math.log(1000));
  return (n / Math.pow(1000, i)).toFixed(i === 0 ? 0 : 1) + " " + units[i];
}
function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => (
    { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]
  ));
}
async function api(path, opts) {
  const res = await fetch(path, opts);
  const body = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(body.error || `Request failed (${res.status})`);
  return body;
}
function toast(msg, kind) {
  const el = document.getElementById("toast");
  el.textContent = msg;
  el.className = "toast " + (kind || "");
  setTimeout(() => el.classList.add("hidden"), 3500);
}

// ---------- storage gauge ----------
function gaugeHtml(storage) {
  const pct = Math.min(100, storage.percent || 0);
  return `
    <div class="gauge">
      <div class="gauge-label">
        <span>💾 <b>${fmtBytes(storage.used)}</b> used of <b>${fmtBytes(storage.total)}</b></span>
        <span>${fmtBytes(storage.available)} free · ${pct.toFixed(0)}%</span>
      </div>
      <div class="bar"><div class="bar-fill" style="width:${pct}%"></div></div>
    </div>`;
}

// ---------- home ----------
function buildOptions() {
  scanOptions = [
    { icon: "🌐", name: "Full Disk Scan", desc: "Comprehensive system-wide analysis", path: config.root_path },
    { icon: "🏠", name: "Home Directory", desc: "Scan your personal files and folders", path: config.home_path },
    { icon: "📁", name: "Custom Path", desc: "Choose a specific directory to scan", custom: true },
  ];
}

function currentPath() {
  const opt = scanOptions[selectedOption];
  return opt.custom ? customPath : opt.path;
}

function renderHome() {
  buildOptions();
  const cards = scanOptions.map((o, i) => `
    <button class="option ${i === selectedOption ? "selected" : ""}" data-opt="${i}">
      <div class="icon">${o.icon}</div>
      <div class="name">${esc(o.name)}</div>
      <div class="desc">${esc(o.desc)}</div>
      ${o.custom ? "" : `<div class="path">${esc(o.path)}</div>`}
    </button>`).join("");

  app.innerHTML = `
    ${gaugeHtml(config.storage)}
    <div class="section-title">🔍 Choose a scan</div>
    <div class="options">${cards}</div>
    <div class="controls">
      <div class="field" id="custom-field" style="display:none">
        <label>Custom path</label>
        <input class="path-input" id="custom-path" placeholder="/path/to/scan" value="${esc(customPath)}" />
      </div>
      <div class="field">
        <label>Min file size (MB)</label>
        <input type="number" id="min-size" min="0" value="${minSize}" />
      </div>
      <div class="field">
        <label>Max depth (0 = unlimited)</label>
        <input type="number" id="max-depth" min="0" value="${maxDepth}" />
      </div>
      <button class="btn btn-primary" id="start-btn">Start Scan →</button>
    </div>`;

  updateTopStats(config.storage);

  app.querySelectorAll(".option").forEach((el) => {
    el.addEventListener("click", () => {
      selectedOption = parseInt(el.dataset.opt, 10);
      renderHome();
    });
  });
  const customField = document.getElementById("custom-field");
  if (scanOptions[selectedOption].custom) customField.style.display = "flex";

  const cp = document.getElementById("custom-path");
  if (cp) cp.addEventListener("input", (e) => (customPath = e.target.value));
  document.getElementById("min-size").addEventListener("input", (e) => (minSize = parseInt(e.target.value, 10) || 0));
  document.getElementById("max-depth").addEventListener("input", (e) => (maxDepth = parseInt(e.target.value, 10) || 0));
  document.getElementById("start-btn").addEventListener("click", startScan);
}

function updateTopStats(storage) {
  if (!storage) { topStats.innerHTML = ""; return; }
  topStats.innerHTML =
    `<span>💾 <b>${fmtBytes(storage.used)}</b> / ${fmtBytes(storage.total)}</span>` +
    `<span><b>${(storage.percent || 0).toFixed(0)}%</b> used</span>`;
}

// ---------- scanning ----------
async function startScan() {
  const path = currentPath();
  if (!path) { toast("Please enter a path to scan", "error"); return; }
  try {
    await api("/api/scan", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, min_size_mb: minSize, max_depth: maxDepth }),
    });
  } catch (e) {
    toast(e.message, "error");
    return;
  }
  renderScanning(path);
  streamProgress();
}

function renderScanning(path) {
  app.innerHTML = `
    <div class="scanning">
      <div class="spinner"></div>
      <h2>Scanning…</h2>
      <div class="scan-stats">
        <div class="scan-stat"><div class="num" id="s-files">0</div><div class="lbl">files</div></div>
        <div class="scan-stat"><div class="num" id="s-dirs">0</div><div class="lbl">directories</div></div>
        <div class="scan-stat"><div class="num" id="s-size">0 B</div><div class="lbl">discovered</div></div>
      </div>
      <div class="scan-path" id="s-path">${esc(path)}</div>
    </div>`;
}

function streamProgress() {
  const es = new EventSource("/api/scan/stream");
  es.onmessage = (ev) => {
    let p;
    try { p = JSON.parse(ev.data); } catch { return; }
    const f = document.getElementById("s-files");
    if (f) {
      f.textContent = p.files.toLocaleString();
      document.getElementById("s-dirs").textContent = p.dirs.toLocaleString();
      document.getElementById("s-size").textContent = fmtBytes(p.size);
      if (p.current_path) document.getElementById("s-path").textContent = p.current_path;
    }
    if (p.complete) {
      es.close();
      loadResults();
    }
  };
  es.onerror = () => { es.close(); loadResults(); };
}

// ---------- results ----------
async function loadResults() {
  try {
    results = await api("/api/results");
  } catch (e) {
    toast("Could not load results: " + e.message, "error");
    renderHome();
    return;
  }
  selected.clear();
  renderResults();
}

function renderResults() {
  const r = results;
  updateTopStats(r.storage);

  const cats = r.categories.map((c) => {
    const pct = r.total_size > 0 ? (c.size / r.total_size) * 100 : 0;
    const chip = c.safe
      ? `<span class="chip safe">safe</span>`
      : `<span class="chip review">review</span>`;
    return `
      <div class="catrow">
        <div class="catrow-top">
          <span>${esc(c.name)} ${chip} <span class="sz">· ${c.count} items</span></span>
          <span class="sz">${fmtBytes(c.size)} · ${pct.toFixed(0)}%</span>
        </div>
        <div class="catbar"><div class="catbar-fill" style="width:${pct}%;background:${c.color}"></div></div>
      </div>`;
  }).join("") || `<div class="empty">No categories.</div>`;

  const maxDir = r.directories.reduce((m, d) => Math.max(m, d.size), 0) || 1;
  const dirs = r.directories.slice(0, 12).map((d) => {
    const pct = (d.size / maxDir) * 100;
    return `
      <div class="catrow">
        <div class="catrow-top"><span class="mono">${esc(d.name)}</span><span class="sz">${fmtBytes(d.size)}</span></div>
        <div class="catbar"><div class="catbar-fill" style="width:${pct}%;background:var(--accent)"></div></div>
      </div>`;
  }).join("") || `<div class="empty">No sub-directories.</div>`;

  const recs = r.recommendations.map((x) => `<li>💡 ${esc(x)}</li>`).join("");

  app.innerHTML = `
    <div class="results-head">
      <div>
        <h1>📊 ${esc(r.scan_path)}</h1>
        <div>${r.total_files.toLocaleString()} files · ${r.total_dirs.toLocaleString()} dirs · ${fmtBytes(r.total_size)} ·
          <span class="savings">${fmtBytes(r.safe_savings)} safe to reclaim</span></div>
      </div>
      <div class="head-actions">
        <button class="btn btn-ghost btn-sm" id="rescan-btn">← New scan</button>
      </div>
    </div>
    ${gaugeHtml(r.storage)}
    <div class="grid2">
      <div class="card"><h3>📂 Category breakdown</h3>${cats}</div>
      <div class="card"><h3>🗂️ Top directories</h3>${dirs}</div>
    </div>
    <div class="card recs"><h3>💡 Recommendations</h3><ul>${recs}</ul></div>
    <div class="card">
      <h3>📄 Largest files (${r.files.length})</h3>
      <div class="toolbar">
        <button class="btn btn-ghost btn-sm" id="sel-safe">Select safe</button>
        <button class="btn btn-ghost btn-sm" id="sel-all">Select all</button>
        <button class="btn btn-ghost btn-sm" id="sel-clear">Clear</button>
        <span class="spacer"></span>
        <span class="sel-count" id="sel-count"></span>
        <button class="btn btn-danger btn-sm" id="del-btn" disabled>Delete selected</button>
      </div>
      <div class="table-wrap">
        <table>
          <thead><tr><th></th><th>Name</th><th>Category</th><th>Modified</th><th style="text-align:right">Size</th></tr></thead>
          <tbody id="file-rows"></tbody>
        </table>
      </div>
    </div>`;

  document.getElementById("rescan-btn").addEventListener("click", boot);
  document.getElementById("sel-safe").addEventListener("click", () => {
    r.files.forEach((f) => { if (f.safe) selected.add(f.path); });
    renderRows();
  });
  document.getElementById("sel-all").addEventListener("click", () => {
    r.files.forEach((f) => selected.add(f.path));
    renderRows();
  });
  document.getElementById("sel-clear").addEventListener("click", () => { selected.clear(); renderRows(); });
  document.getElementById("del-btn").addEventListener("click", confirmDelete);

  renderRows();
}

function renderRows() {
  const rows = results.files.map((f) => {
    const checked = selected.has(f.path) ? "checked" : "";
    const flags = (f.is_system ? "⚙️" : "") + (f.is_hidden ? "◌" : "");
    return `
      <tr>
        <td><input type="checkbox" data-path="${esc(f.path)}" ${checked} /></td>
        <td class="name" title="${esc(f.path)}">${esc(f.name)} <span class="flag">${flags}</span></td>
        <td><span class="cat-dot" style="background:${f.color}"></span>${esc(f.category)}</td>
        <td class="mono">${esc(f.modified)}</td>
        <td class="size">${fmtBytes(f.size)}</td>
      </tr>`;
  }).join("");
  document.getElementById("file-rows").innerHTML = rows ||
    `<tr><td colspan="5" class="empty">No files above the size threshold.</td></tr>`;

  document.querySelectorAll('#file-rows input[type="checkbox"]').forEach((cb) => {
    cb.addEventListener("change", () => {
      if (cb.checked) selected.add(cb.dataset.path);
      else selected.delete(cb.dataset.path);
      updateSelCount();
    });
  });
  updateSelCount();
}

function selectedSize() {
  const map = new Map(results.files.map((f) => [f.path, f.size]));
  let sum = 0;
  selected.forEach((p) => (sum += map.get(p) || 0));
  return sum;
}

function updateSelCount() {
  const n = selected.size;
  document.getElementById("sel-count").textContent =
    n ? `${n} selected · ${fmtBytes(selectedSize())}` : "";
  document.getElementById("del-btn").disabled = n === 0;
}

// ---------- delete ----------
function confirmDelete() {
  const n = selected.size;
  if (!n) return;
  const modal = document.getElementById("modal");
  document.getElementById("modal-body").innerHTML =
    `Permanently delete <b>${n}</b> item${n > 1 ? "s" : ""} (${fmtBytes(selectedSize())})? This cannot be undone.`;
  modal.classList.remove("hidden");

  const cancel = document.getElementById("modal-cancel");
  const confirm = document.getElementById("modal-confirm");
  const close = () => {
    modal.classList.add("hidden");
    cancel.onclick = null;
    confirm.onclick = null;
  };
  cancel.onclick = close;
  confirm.onclick = async () => {
    close();
    await doDelete();
  };
}

async function doDelete() {
  const paths = Array.from(selected);
  try {
    const res = await api("/api/delete", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ paths }),
    });
    toast(`Deleted ${res.deleted}/${paths.length} · freed ${fmtBytes(res.freed)}`, "success");
    selected.clear();
    await loadResults();
  } catch (e) {
    toast("Delete failed: " + e.message, "error");
  }
}

// ---------- boot ----------
async function boot() {
  config = await api("/api/config");
  minSize = config.min_size_mb;
  maxDepth = config.max_depth;
  customPath = config.default_path;
  selectedOption = 0;
  renderHome();
}

boot().catch((e) => {
  app.innerHTML = `<div class="empty">Failed to load: ${esc(e.message)}</div>`;
});
