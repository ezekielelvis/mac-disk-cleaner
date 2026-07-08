// Cleaner page — scan a path, review a categorized breakdown, and delete files.
// This is the original Disk Cleaner workflow, adapted into a routed page.

import { getConfig, startScan, getResults, deletePaths, toast } from "../lib/api.js";
import { fmtBytes, esc } from "../lib/format.js";

export async function mount(view) {
  const state = {
    config: null,
    options: [],
    selectedOption: 0,
    customPath: "",
    minSize: 1,
    maxDepth: 0,
    results: null,
    selected: new Set(),
    es: null, // active EventSource during a scan
  };

  view.innerHTML = `<div class="empty">Loading…</div>`;

  try {
    state.config = await getConfig();
  } catch (e) {
    view.innerHTML = `<div class="empty">Failed to load: ${esc(e.message)}</div>`;
    return () => {};
  }
  state.minSize = state.config.min_size_mb;
  state.maxDepth = state.config.max_depth;
  state.customPath = state.config.default_path;
  renderHome();

  // ---------- storage gauge ----------
  function gaugeHtml(storage) {
    const pct = Math.min(100, storage.percent || 0);
    return `
      <div class="gauge card">
        <div class="gauge-label">
          <span><b>${fmtBytes(storage.used)}</b> used of <b>${fmtBytes(storage.total)}</b></span>
          <span>${fmtBytes(storage.available)} free · ${pct.toFixed(0)}%</span>
        </div>
        <div class="bar"><div class="bar-fill" style="width:${pct}%"></div></div>
      </div>`;
  }

  // ---------- home ----------
  function buildOptions() {
    state.options = [
      { name: "Full Disk Scan", desc: "Comprehensive system-wide analysis", path: state.config.root_path },
      { name: "Home Directory", desc: "Scan your personal files and folders", path: state.config.home_path },
      { name: "Custom Path", desc: "Choose a specific directory to scan", custom: true },
    ];
  }

  function currentPath() {
    const opt = state.options[state.selectedOption];
    return opt.custom ? state.customPath : opt.path;
  }

  function renderHome() {
    buildOptions();
    const cards = state.options
      .map(
        (o, i) => `
      <button type="button" class="option ${i === state.selectedOption ? "selected" : ""}" data-opt="${i}">
        <div class="name">${esc(o.name)}</div>
        <div class="desc">${esc(o.desc)}</div>
        ${o.custom ? "" : `<div class="path">${esc(o.path)}</div>`}
      </button>`
      )
      .join("");

    view.innerHTML = `
      <div class="page-head">
        <h1>Cleaner</h1>
        <div class="sub">Scan a location, then review and reclaim disk space safely.</div>
      </div>
      ${gaugeHtml(state.config.storage)}
      <div class="section-title">Choose a scan</div>
      <div class="options">${cards}</div>
      <div class="controls card">
        <div class="field" id="custom-field" style="display:none">
          <label>Custom path</label>
          <input class="path-input" id="custom-path" placeholder="/path/to/scan" value="${esc(state.customPath)}" />
        </div>
        <div class="field">
          <label>Min file size (MB)</label>
          <input type="number" id="min-size" min="0" value="${state.minSize}" />
        </div>
        <div class="field">
          <label>Max depth (0 = unlimited)</label>
          <input type="number" id="max-depth" min="0" value="${state.maxDepth}" />
        </div>
        <button type="button" class="btn btn-primary" id="start-btn">Start Scan →</button>
      </div>`;

    view.querySelectorAll(".option").forEach((el) => {
      el.addEventListener("click", () => {
        state.selectedOption = parseInt(el.dataset.opt, 10);
        renderHome();
      });
    });
    if (state.options[state.selectedOption].custom) {
      view.querySelector("#custom-field").style.display = "flex";
    }
    const cp = view.querySelector("#custom-path");
    if (cp) cp.addEventListener("input", (e) => (state.customPath = e.target.value));
    view.querySelector("#min-size").addEventListener("input", (e) => (state.minSize = parseInt(e.target.value, 10) || 0));
    view.querySelector("#max-depth").addEventListener("input", (e) => (state.maxDepth = parseInt(e.target.value, 10) || 0));
    view.querySelector("#start-btn").addEventListener("click", start);
  }

  // ---------- scanning ----------
  async function start() {
    const path = currentPath();
    if (!path) { toast("Please enter a path to scan", "error"); return; }
    try {
      await startScan({ path, min_size_mb: state.minSize, max_depth: state.maxDepth });
    } catch (e) {
      toast(e.message, "error");
      return;
    }
    renderScanning(path);
    streamProgress();
  }

  function renderScanning(path) {
    view.innerHTML = `
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
    state.es = es;
    es.onmessage = (ev) => {
      let p;
      try { p = JSON.parse(ev.data); } catch { return; }
      const f = view.querySelector("#s-files");
      if (f) {
        f.textContent = p.files.toLocaleString();
        view.querySelector("#s-dirs").textContent = p.dirs.toLocaleString();
        view.querySelector("#s-size").textContent = fmtBytes(p.size);
        if (p.current_path) view.querySelector("#s-path").textContent = p.current_path;
      }
      if (p.complete) { es.close(); state.es = null; loadResults(); }
    };
    es.onerror = () => { es.close(); state.es = null; loadResults(); };
  }

  // ---------- results ----------
  async function loadResults() {
    try {
      state.results = await getResults();
    } catch (e) {
      toast("Could not load results: " + e.message, "error");
      renderHome();
      return;
    }
    state.selected.clear();
    renderResults();
  }

  function renderResults() {
    const r = state.results;
    const cats = r.categories
      .map((c) => {
        const pct = r.total_size > 0 ? (c.size / r.total_size) * 100 : 0;
        const chip = c.safe ? `<span class="pill green">safe</span>` : `<span class="pill amber">review</span>`;
        return `
        <div class="catrow">
          <div class="catrow-top">
            <span>${esc(c.name)} ${chip} <span class="sz">· ${c.count} items</span></span>
            <span class="sz">${fmtBytes(c.size)} · ${pct.toFixed(0)}%</span>
          </div>
          <div class="catbar"><div class="catbar-fill" style="width:${pct}%;background:${c.color}"></div></div>
        </div>`;
      })
      .join("") || `<div class="empty">No categories.</div>`;

    const maxDir = r.directories.reduce((m, d) => Math.max(m, d.size), 0) || 1;
    const dirs = r.directories
      .slice(0, 12)
      .map((d) => {
        const pct = (d.size / maxDir) * 100;
        return `
        <div class="catrow">
          <div class="catrow-top"><span class="mono">${esc(d.name)}</span><span class="sz">${fmtBytes(d.size)}</span></div>
          <div class="catbar"><div class="catbar-fill" style="width:${pct}%;background:var(--accent)"></div></div>
        </div>`;
      })
      .join("") || `<div class="empty">No sub-directories.</div>`;

    const recs = r.recommendations.map((x) => `<li>${esc(x)}</li>`).join("");

    view.innerHTML = `
      <div class="page-head page-head-row">
        <div>
          <h1>Scan results</h1>
          <div class="sub mono">${esc(r.scan_path)}</div>
          <div class="sub">${r.total_files.toLocaleString()} files · ${r.total_dirs.toLocaleString()} dirs · ${fmtBytes(r.total_size)} ·
            <span class="savings">${fmtBytes(r.safe_savings)} safe to reclaim</span></div>
        </div>
        <button type="button" class="btn btn-ghost btn-sm" id="rescan-btn">← New scan</button>
      </div>
      ${gaugeHtml(r.storage)}
      <div class="grid2">
        <div class="card"><h3>Category breakdown</h3>${cats}</div>
        <div class="card"><h3>Top directories</h3>${dirs}</div>
      </div>
      <div class="card recs"><h3>Recommendations</h3><ul>${recs}</ul></div>
      <div class="card">
        <h3>Largest files (${r.files.length})</h3>
        <div class="toolbar">
          <button type="button" class="btn btn-ghost btn-sm" id="sel-safe">Select safe</button>
          <button type="button" class="btn btn-ghost btn-sm" id="sel-all">Select all</button>
          <button type="button" class="btn btn-ghost btn-sm" id="sel-clear">Clear</button>
          <span class="spacer"></span>
          <span class="sel-count" id="sel-count"></span>
          <button type="button" class="btn btn-danger btn-sm" id="del-btn" disabled>Delete selected</button>
        </div>
        <div class="table-wrap">
          <table>
            <thead><tr><th></th><th>Name</th><th>Category</th><th>Modified</th><th style="text-align:right">Size</th></tr></thead>
            <tbody id="file-rows"></tbody>
          </table>
        </div>
      </div>`;

    view.querySelector("#rescan-btn").addEventListener("click", renderHome);
    view.querySelector("#sel-safe").addEventListener("click", () => {
      r.files.forEach((f) => { if (f.safe) state.selected.add(f.path); });
      renderRows();
    });
    view.querySelector("#sel-all").addEventListener("click", () => {
      r.files.forEach((f) => state.selected.add(f.path));
      renderRows();
    });
    view.querySelector("#sel-clear").addEventListener("click", () => { state.selected.clear(); renderRows(); });
    view.querySelector("#del-btn").addEventListener("click", confirmDelete);
    renderRows();
  }

  function renderRows() {
    const rows = state.results.files
      .map((f) => {
        const checked = state.selected.has(f.path) ? "checked" : "";
        const flags = [f.is_system ? "sys" : "", f.is_hidden ? "hidden" : ""].filter(Boolean).join(" ");
        return `
        <tr>
          <td><input type="checkbox" data-path="${esc(f.path)}" ${checked} /></td>
          <td class="name" title="${esc(f.path)}">${esc(f.name)} <span class="flag">${flags}</span></td>
          <td><span class="cat-dot" style="background:${f.color}"></span>${esc(f.category)}</td>
          <td class="mono">${esc(f.modified)}</td>
          <td class="size">${fmtBytes(f.size)}</td>
        </tr>`;
      })
      .join("");
    view.querySelector("#file-rows").innerHTML =
      rows || `<tr><td colspan="5" class="empty">No files above the size threshold.</td></tr>`;

    view.querySelectorAll('#file-rows input[type="checkbox"]').forEach((cb) => {
      cb.addEventListener("change", () => {
        if (cb.checked) state.selected.add(cb.dataset.path);
        else state.selected.delete(cb.dataset.path);
        updateSelCount();
      });
    });
    updateSelCount();
  }

  function selectedSize() {
    const map = new Map(state.results.files.map((f) => [f.path, f.size]));
    let sum = 0;
    state.selected.forEach((p) => (sum += map.get(p) || 0));
    return sum;
  }

  function updateSelCount() {
    const n = state.selected.size;
    view.querySelector("#sel-count").textContent = n ? `${n} selected · ${fmtBytes(selectedSize())}` : "";
    view.querySelector("#del-btn").disabled = n === 0;
  }

  // ---------- delete ----------
  function confirmDelete() {
    const n = state.selected.size;
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
    confirm.onclick = async () => { close(); await doDelete(); };
  }

  async function doDelete() {
    const paths = Array.from(state.selected);
    try {
      const res = await deletePaths(paths);
      toast(`Deleted ${res.deleted}/${paths.length} · freed ${fmtBytes(res.freed)}`, "success");
      state.selected.clear();
      await loadResults();
    } catch (e) {
      toast("Delete failed: " + e.message, "error");
    }
  }

  // cleanup on navigation away: stop any in-flight scan stream + hide modal
  return () => {
    if (state.es) { state.es.close(); state.es = null; }
    document.getElementById("modal").classList.add("hidden");
  };
}
