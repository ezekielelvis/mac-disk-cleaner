// System page — OS, health, processor, memory and storage. System Health and
// the Processor are shown as Chart.js bar charts. The DOM skeleton is built once
// and then updated in place every few seconds, so the charts don't flicker.
//
/* global Chart */ // provided by the vendored UMD build loaded in index.html

import { getSystem } from "../lib/api.js";
import { fmtBytes, fmtUptime, fmtDate, fmtPct, esc } from "../lib/format.js";

const POLL_MS = 3000;
const GRID = "#eeeeee";
const ACCENT = "#4f46e5";

export async function mount(view) {
  view.innerHTML = `
    <div class="page-head">
      <h1>System</h1>
      <div class="sub">Hardware, operating system and health at a glance.</div>
    </div>
    <div id="sys-body"><div class="empty">Loading system information…</div></div>
  `;
  const body = view.querySelector("#sys-body");
  const charts = {};
  let built = false;

  async function refresh() {
    let s;
    try {
      s = await getSystem();
    } catch (e) {
      destroyCharts(charts);
      built = false;
      body.innerHTML = `<div class="empty">Could not read system info: ${esc(e.message)}</div>`;
      return;
    }
    if (!built) {
      body.innerHTML = skeleton(s);
      createCharts(s, charts, body);
      built = true;
    }
    apply(s, charts, body);
  }

  await refresh();
  const timer = setInterval(refresh, POLL_MS);
  return () => {
    clearInterval(timer);
    destroyCharts(charts);
  };
}

// ---------- structure (built once) ----------
function skeleton(s) {
  return `
    <div class="sys-grid">
      <div class="card"><h3>Operating System</h3><div class="kv" id="os-kv"></div></div>
      <div class="card health-card">
        <div class="health-head"><h3>System Health</h3><span id="health-status"></span></div>
        <div id="health-text"></div>
        <div class="sys-chart health-chart"><canvas id="health-bar"></canvas></div>
      </div>
    </div>
    <div class="card">
      <h3>Processor</h3>
      <div class="cpu-brand">${esc(s.cpu.brand || "Unknown CPU")}</div>
      <div class="kv" id="cpu-kv"></div>
      <div class="sys-chart core-chart"><canvas id="core-bar"></canvas></div>
    </div>
    <div class="card"><h3>Memory</h3><div class="mem-row" id="mem-body"></div></div>
    <div class="card"><h3>Storage Devices</h3><div class="disks" id="disks-body"></div></div>
  `;
}

// ---------- charts ----------
function healthValues(s) {
  const maxDisk = (s.disks || []).reduce((m, d) => Math.max(m, d.percent || 0), 0);
  return {
    labels: ["CPU", "RAM", "Swap", "Storage"],
    values: [s.cpu.usage || 0, s.memory.percent || 0, s.memory.swap_percent || 0, maxDisk],
  };
}

function createCharts(s, charts, body) {
  const hv = healthValues(s);
  charts.health = new Chart(body.querySelector("#health-bar"), {
    type: "bar",
    data: {
      labels: hv.labels,
      datasets: [{ data: hv.values, backgroundColor: hv.values.map(barColor), borderWidth: 0, barThickness: 20 }],
    },
    options: barOpts("y"),
  });

  const cores = s.cpu.per_core || [];
  charts.core = new Chart(body.querySelector("#core-bar"), {
    type: "bar",
    data: {
      labels: cores.map((_, i) => `#${i}`),
      datasets: [{ data: cores.map((v) => Math.min(100, v)), backgroundColor: ACCENT, borderWidth: 0 }],
    },
    options: barOpts("x"),
  });
}

function barOpts(indexAxis) {
  const pctAxis = {
    min: 0, max: 100,
    grid: { color: GRID }, border: { display: false },
    ticks: { callback: (v) => `${v}%`, font: { size: 10 }, color: "#9a9a9a", stepSize: 25 },
  };
  const catAxis = {
    grid: { display: false }, border: { display: false },
    ticks: { font: { size: 11 }, color: "#626262" },
  };
  return {
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    indexAxis,
    scales: indexAxis === "y" ? { x: pctAxis, y: catAxis } : { y: pctAxis, x: catAxis },
    plugins: {
      legend: { display: false },
      tooltip: { callbacks: { label: (ctx) => `${ctx.raw.toFixed(0)}%` } },
    },
  };
}

function destroyCharts(charts) {
  Object.values(charts).forEach((c) => { try { c.destroy(); } catch (_) {} });
}

// ---------- per-poll updates ----------
function apply(s, charts, body) {
  body.querySelector("#os-kv").innerHTML = kvRows([
    ["OS", s.os.name],
    ["Version", s.os.os_version],
    ["Kernel", s.os.kernel],
    ["Architecture", s.os.arch],
    ["Hostname", s.os.hostname],
    ["Uptime", fmtUptime(s.os.uptime)],
    ["Booted", fmtDate(s.os.boot_time)],
  ]);

  body.querySelector("#health-status").innerHTML = statusPill(s.health.status);
  body.querySelector("#health-text").innerHTML = healthText(s.health);
  const hv = healthValues(s);
  charts.health.data.datasets[0].data = hv.values;
  charts.health.data.datasets[0].backgroundColor = hv.values.map(barColor);
  charts.health.update("none");

  body.querySelector("#cpu-kv").innerHTML = kvRows([
    ["Cores", `${s.cpu.physical_cores} physical · ${s.cpu.logical_cores} logical`],
    ["Frequency", `${(s.cpu.frequency_mhz / 1000).toFixed(2)} GHz`],
    ["Total load", fmtPct(s.cpu.usage)],
    ["Load avg", `${s.load.one.toFixed(2)} · ${s.load.five.toFixed(2)} · ${s.load.fifteen.toFixed(2)}`],
  ]);
  const cores = s.cpu.per_core || [];
  charts.core.data.labels = cores.map((_, i) => `#${i}`);
  charts.core.data.datasets[0].data = cores.map((v) => Math.min(100, v));
  charts.core.update("none");

  body.querySelector("#mem-body").innerHTML =
    meter("RAM", s.memory.used, s.memory.total, s.memory.percent, "var(--ink)") +
    meter("Swap", s.memory.swap_used, s.memory.swap_total, s.memory.swap_percent, "var(--ink-soft)");

  body.querySelector("#disks-body").innerHTML =
    (s.disks || []).map(diskRow).join("") || `<div class="empty">No disks reported.</div>`;
}

// ---------- small render helpers ----------
function kvRows(rows) {
  return rows.map(([k, v]) => `<div class="k">${esc(k)}</div><div class="v">${esc(v || "—")}</div>`).join("");
}

function statusPill(status) {
  const map = { healthy: ["green", "Healthy"], warning: ["amber", "Attention"], critical: ["red", "Critical"] };
  const [cls, label] = map[status] || map.healthy;
  return `<span class="pill ${cls}">${label}</span>`;
}

function healthText(health) {
  return (health.issues || []).length
    ? `<ul class="issue-list">${health.issues.map((i) => `<li>${esc(i)}</li>`).join("")}</ul>`
    : `<div class="all-good">No problems detected.</div>`;
}

function meter(label, used, total, pct, color) {
  return `
    <div class="meter">
      <div class="meter-head">
        <span>${label}</span>
        <span class="meter-nums">${fmtBytes(used)} / ${fmtBytes(total)} · ${fmtPct(pct)}</span>
      </div>
      <div class="track"><div class="fill" style="width:${Math.min(100, pct)}%;background:${color}"></div></div>
    </div>`;
}

function diskRow(d) {
  return `
    <div class="disk-row">
      <div class="disk-main">
        <div class="disk-mount mono">${esc(d.mount)}</div>
        <div class="disk-meta">${esc(d.fs)} · ${esc(d.kind)} · ${esc(d.name) || "disk"}</div>
      </div>
      <div class="disk-bar"><div class="fill" style="width:${Math.min(100, d.percent)}%;background:${barColor(d.percent)}"></div></div>
      <div class="disk-nums">${fmtBytes(d.used)} / ${fmtBytes(d.total)} <span class="disk-pct">${fmtPct(d.percent)}</span></div>
    </div>`;
}

function barColor(pct) {
  if (pct >= 90) return "#dc2626";
  if (pct >= 75) return "#d97706";
  return "#16a34a";
}
