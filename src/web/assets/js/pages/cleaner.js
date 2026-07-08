// Cleaner page — scan a path, then explore results visually:
//
//   home  →  scanning (animated polar loader)  →  overview (interactive polar of
//   categories)  →  detail (radar of files + interactive polar of folders, with
//   the smart selection + delete tools).
//
// Clicking a slice on the overview polar drills into that category. On the
// detail page, clicking a folder slice filters the file list to that folder.
//
/* global Chart */ // provided by the vendored UMD build loaded in index.html

import { getConfig, startScan, getResults, deletePaths, toast } from "../lib/api.js";
import { fmtBytes, esc } from "../lib/format.js";

// Fallback palette for folder slices (categories carry their own colors).
const PALETTE = [
  "#4f46e5", "#16a34a", "#d97706", "#0891b2", "#db2777",
  "#7c3aed", "#0d9488", "#dc2626", "#ca8a04", "#2563eb",
];
const GRID = "#eeeeee";

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
    es: null,          // active EventSource during a scan
    charts: [],        // live Chart.js instances to tear down
    scanTimer: null,   // animation interval for the scanning polar
    detailCategory: null, // category name currently drilled into
    folderFilter: null,   // directory path narrowing the detail file list
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

  // ---------- chart lifecycle ----------
  function destroyCharts() {
    state.charts.forEach((c) => { try { c.destroy(); } catch (_) {} });
    state.charts = [];
  }
  function track(chart) { state.charts.push(chart); return chart; }

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
    destroyCharts();
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
    destroyCharts();
    view.innerHTML = `
      <div class="scanning">
        <div class="scan-chart"><canvas id="scan-polar"></canvas></div>
        <h2>Scanning…</h2>
        <div class="scan-stats">
          <div class="scan-stat"><div class="num" id="s-files">0</div><div class="lbl">files</div></div>
          <div class="scan-stat"><div class="num" id="s-dirs">0</div><div class="lbl">directories</div></div>
          <div class="scan-stat"><div class="num" id="s-size">0 B</div><div class="lbl">discovered</div></div>
        </div>
        <div class="scan-path" id="s-path">${esc(path)}</div>
      </div>`;

    // Animated polar loader: segments pulse while the scan runs. Replaced by the
    // real, interactive category polar once results are in.
    const n = 7;
    const chart = track(new Chart(view.querySelector("#scan-polar"), {
      type: "polarArea",
      data: {
        labels: Array.from({ length: n }, () => ""),
        datasets: [{
          data: randData(n),
          backgroundColor: PALETTE.slice(0, n).map((c) => hexToRgba(c, 0.55)),
          borderColor: PALETTE.slice(0, n),
          borderWidth: 1,
        }],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: { duration: 650 },
        plugins: { legend: { display: false }, tooltip: { enabled: false } },
        scales: { r: { ticks: { display: false }, grid: { color: GRID }, angleLines: { color: GRID } } },
      },
    }));
    state.scanTimer = setInterval(() => {
      chart.data.datasets[0].data = randData(n);
      chart.update();
    }, 700);
  }

  function stopScanAnim() {
    if (state.scanTimer) { clearInterval(state.scanTimer); state.scanTimer = null; }
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
      if (p.complete) { es.close(); state.es = null; stopScanAnim(); loadResults(); }
    };
    es.onerror = () => { es.close(); state.es = null; stopScanAnim(); loadResults(); };
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
    state.detailCategory = null;
    state.folderFilter = null;
    renderOverview();
  }

  // ---------- overview: interactive polar of categories ----------
  function renderOverview() {
    destroyCharts();
    const r = state.results;
    const cats = r.categories.filter((c) => c.size > 0);
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
      <div class="card">
        <h3>Categories — click a slice to explore</h3>
        <div class="polar-wrap"><canvas id="cat-polar"></canvas></div>
      </div>
      <div class="card recs"><h3>Recommendations</h3><ul>${recs || "<li>Nothing to flag.</li>"}</ul></div>`;

    view.querySelector("#rescan-btn").addEventListener("click", renderHome);

    if (!cats.length) return;
    track(new Chart(view.querySelector("#cat-polar"), {
      type: "polarArea",
      data: {
        labels: cats.map((c) => c.name),
        datasets: [{
          data: cats.map((c) => c.size),
          backgroundColor: cats.map((c) => hexToRgba(c.color, 0.6)),
          borderColor: cats.map((c) => c.color),
          borderWidth: 1,
        }],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { position: "right", labels: { boxWidth: 12, font: { size: 11 } } },
          tooltip: { callbacks: { label: (ctx) => `${ctx.label}: ${fmtBytes(ctx.raw)}` } },
        },
        scales: { r: { ticks: { display: false }, grid: { color: GRID }, angleLines: { color: GRID } } },
        onHover: (evt, els) => { evt.native.target.style.cursor = els.length ? "pointer" : "default"; },
        onClick: (evt, els) => {
          if (!els.length) return;
          state.detailCategory = cats[els[0].index].name;
          state.folderFilter = null;
          renderDetail();
        },
      },
    }));
  }

  // ---------- detail: radar of files + interactive polar of folders ----------
  function renderDetail() {
    destroyCharts();
    const catName = state.detailCategory;
    const catFiles = state.results.files.filter((f) => f.category === catName);
    const files = state.folderFilter
      ? catFiles.filter((f) => folderOf(f.path) === state.folderFilter)
      : catFiles;

    const catSize = catFiles.reduce((s, f) => s + f.size, 0);
    const filterChip = state.folderFilter
      ? `<span class="pill" id="clear-folder" style="cursor:pointer">${esc(baseName(state.folderFilter))} ✕</span>`
      : "";

    view.innerHTML = `
      <div class="page-head page-head-row">
        <div>
          <h1>${esc(catName)}</h1>
          <div class="sub">${catFiles.length.toLocaleString()} files · ${fmtBytes(catSize)} ${filterChip}</div>
        </div>
        <button type="button" class="btn btn-ghost btn-sm" id="back-btn">← Back to overview</button>
      </div>
      <div class="grid2">
        <div class="card">
          <h3>Files &amp; sizes</h3>
          <div class="polar-wrap"><canvas id="file-radar"></canvas></div>
        </div>
        <div class="card">
          <h3>Folders — click to filter</h3>
          <div class="polar-wrap"><canvas id="folder-polar"></canvas></div>
        </div>
      </div>
      <div class="card">
        <h3>Files (${files.length})</h3>
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
            <thead><tr><th></th><th>Name</th><th>Modified</th><th style="text-align:right">Size</th></tr></thead>
            <tbody id="file-rows"></tbody>
          </table>
        </div>
      </div>`;

    view.querySelector("#back-btn").addEventListener("click", renderOverview);
    const clearFolder = view.querySelector("#clear-folder");
    if (clearFolder) clearFolder.addEventListener("click", () => { state.folderFilter = null; renderDetail(); });

    // radar: the biggest files in the current view, by size
    const topFiles = files.slice().sort((a, b) => b.size - a.size).slice(0, 7);
    if (topFiles.length) {
      track(new Chart(view.querySelector("#file-radar"), {
        type: "radar",
        data: {
          labels: topFiles.map((f) => truncate(f.name, 16)),
          datasets: [{
            label: "Size",
            data: topFiles.map((f) => f.size),
            backgroundColor: "rgba(79,70,229,0.15)",
            borderColor: "#4f46e5",
            borderWidth: 1.5,
            pointBackgroundColor: "#4f46e5",
            pointRadius: 3,
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            legend: { display: false },
            tooltip: { callbacks: { label: (ctx) => fmtBytes(topFiles[ctx.dataIndex].size) } },
          },
          scales: { r: { ticks: { display: false }, grid: { color: GRID }, angleLines: { color: GRID }, pointLabels: { font: { size: 9 } } } },
        },
      }));
    }

    // folders: aggregate this category's files by parent directory, interactive
    const folders = foldersFor(catFiles);
    if (folders.length) {
      track(new Chart(view.querySelector("#folder-polar"), {
        type: "polarArea",
        data: {
          labels: folders.map((d) => d.name),
          datasets: [{
            data: folders.map((d) => d.size),
            backgroundColor: folders.map((d, i) => hexToRgba(colorFor(d.dir, i), state.folderFilter === d.dir ? 0.9 : 0.55)),
            borderColor: folders.map((d, i) => colorFor(d.dir, i)),
            borderWidth: folders.map((d) => (state.folderFilter === d.dir ? 2 : 1)),
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            legend: { position: "right", labels: { boxWidth: 12, font: { size: 11 } } },
            tooltip: { callbacks: { label: (ctx) => `${ctx.label}: ${fmtBytes(ctx.raw)}` } },
          },
          scales: { r: { ticks: { display: false }, grid: { color: GRID }, angleLines: { color: GRID } } },
          onHover: (evt, els) => { evt.native.target.style.cursor = els.length ? "pointer" : "default"; },
          onClick: (evt, els) => {
            if (!els.length) return;
            const dir = folders[els[0].index].dir;
            state.folderFilter = state.folderFilter === dir ? null : dir; // toggle
            renderDetail();
          },
        },
      }));
    }

    // smart selection over the currently visible files
    const selSafe = view.querySelector("#sel-safe");
    const selAll = view.querySelector("#sel-all");
    selSafe.addEventListener("click", () => { files.forEach((f) => { if (f.safe) state.selected.add(f.path); }); renderRows(files); });
    selAll.addEventListener("click", () => { files.forEach((f) => state.selected.add(f.path)); renderRows(files); });
    view.querySelector("#sel-clear").addEventListener("click", () => { files.forEach((f) => state.selected.delete(f.path)); renderRows(files); });
    view.querySelector("#del-btn").addEventListener("click", confirmDelete);
    renderRows(files);
  }

  function renderRows(files) {
    const rows = files
      .map((f) => {
        const checked = state.selected.has(f.path) ? "checked" : "";
        const flags = [f.is_system ? "sys" : "", f.is_hidden ? "hidden" : ""].filter(Boolean).join(" ");
        return `
        <tr>
          <td><input type="checkbox" data-path="${esc(f.path)}" ${checked} /></td>
          <td class="name" title="${esc(f.path)}">${esc(f.name)} <span class="flag">${flags}</span></td>
          <td class="mono">${esc(f.modified)}</td>
          <td class="size">${fmtBytes(f.size)}</td>
        </tr>`;
      })
      .join("");
    view.querySelector("#file-rows").innerHTML =
      rows || `<tr><td colspan="4" class="empty">No files here.</td></tr>`;

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
    const el = view.querySelector("#sel-count");
    if (el) el.textContent = n ? `${n} selected · ${fmtBytes(selectedSize())}` : "";
    const del = view.querySelector("#del-btn");
    if (del) del.disabled = n === 0;
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
      // Reload the fresh result set, returning to the overview.
      await loadResults();
    } catch (e) {
      toast("Delete failed: " + e.message, "error");
    }
  }

  // ---------- small helpers ----------
  function foldersFor(catFiles) {
    const map = new Map();
    catFiles.forEach((f) => {
      const dir = folderOf(f.path);
      const cur = map.get(dir) || { dir, name: baseName(dir), size: 0, count: 0 };
      cur.size += f.size; cur.count++;
      map.set(dir, cur);
    });
    return [...map.values()].sort((a, b) => b.size - a.size).slice(0, 8);
  }

  // cleanup on navigation away: stop scan stream, animation, charts + modal
  return () => {
    if (state.es) { state.es.close(); state.es = null; }
    stopScanAnim();
    destroyCharts();
    document.getElementById("modal").classList.add("hidden");
  };
}

// ---------- module-level pure helpers ----------
function folderOf(path) {
  const i = path.lastIndexOf("/");
  return i > 0 ? path.slice(0, i) : "/";
}
function baseName(path) {
  const parts = path.split("/").filter(Boolean);
  return parts.length ? parts[parts.length - 1] : path;
}
function truncate(s, n) {
  return s.length > n ? s.slice(0, n - 1) + "…" : s;
}
function colorFor(_dir, i) {
  return PALETTE[i % PALETTE.length];
}
function randData(n) {
  return Array.from({ length: n }, () => 4 + Math.random() * 10);
}
function hexToRgba(hex, a) {
  const s = hex.replace("#", "");
  const n = parseInt(s.length === 3 ? s.replace(/(.)/g, "$1$1") : s, 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}
