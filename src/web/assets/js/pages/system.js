// System page — a static-ish snapshot of OS, CPU, memory, disks and health,
// refreshed every few seconds. All data comes from /api/system (sysinfo).

import { getSystem } from "../lib/api.js";
import { fmtBytes, fmtUptime, fmtDate, fmtPct, esc } from "../lib/format.js";

const POLL_MS = 3000;

export async function mount(view) {
  view.innerHTML = `
    <div class="page-head">
      <h1>System</h1>
      <div class="sub">Hardware, operating system and health at a glance.</div>
    </div>
    <div id="sys-body"><div class="empty">Loading system information…</div></div>
  `;
  const body = view.querySelector("#sys-body");

  async function refresh() {
    let s;
    try { s = await getSystem(); } catch (e) {
      body.innerHTML = `<div class="empty">Could not read system info: ${esc(e.message)}</div>`;
      return;
    }
    body.innerHTML = render(s);
  }

  await refresh();
  const timer = setInterval(refresh, POLL_MS);
  return () => clearInterval(timer);
}

function render(s) {
  // OS information comes first, then hardware, memory, storage and health.
  return `
    <div class="sys-grid">
      ${osCard(s.os)}
      ${healthCard(s.health, s.temperatures)}
    </div>
    ${cpuCard(s.cpu, s.load)}
    ${memoryCard(s.memory)}
    ${disksCard(s.disks)}
  `;
}

function statusPill(status) {
  const map = { healthy: ["green", "Healthy"], warning: ["amber", "Attention"], critical: ["red", "Critical"] };
  const [cls, label] = map[status] || map.healthy;
  return `<span class="pill ${cls}">${label}</span>`;
}

function healthCard(health, temps) {
  const issues = (health.issues || []).length
    ? `<ul class="issue-list">${health.issues.map((i) => `<li>${esc(i)}</li>`).join("")}</ul>`
    : `<div class="all-good">No problems detected.</div>`;
  const chips = (temps || []).length
    ? `<div class="temp-row">${temps
        .slice(0, 8)
        .map((t) => `<span class="temp-chip">${esc(t.label)} <b>${t.celsius.toFixed(0)}°C</b></span>`)
        .join("")}</div>`
    : "";
  return `
    <div class="card health-card">
      <div class="health-head">
        <h3>System Health</h3>
        ${statusPill(health.status)}
      </div>
      ${issues}
      ${chips}
    </div>`;
}

function osCard(os) {
  const rows = [
    ["OS", os.name],
    ["Version", os.os_version],
    ["Kernel", os.kernel],
    ["Architecture", os.arch],
    ["Hostname", os.hostname],
    ["Uptime", fmtUptime(os.uptime)],
    ["Booted", fmtDate(os.boot_time)],
  ];
  return kvCard("Operating System", rows);
}

function cpuCard(cpu, load) {
  const cores = (cpu.per_core || [])
    .map(
      (u, i) => `
      <div class="core">
        <div class="core-label">#${i}</div>
        <div class="core-bar"><div class="core-fill" style="width:${Math.min(100, u).toFixed(0)}%"></div></div>
        <div class="core-val">${fmtPct(u)}</div>
      </div>`
    )
    .join("");
  return `
    <div class="card">
      <h3>Processor</h3>
      <div class="cpu-brand">${esc(cpu.brand || "Unknown CPU")}</div>
      <div class="kv">
        <div class="k">Cores</div><div class="v">${cpu.physical_cores} physical · ${cpu.logical_cores} logical</div>
        <div class="k">Frequency</div><div class="v">${(cpu.frequency_mhz / 1000).toFixed(2)} GHz</div>
        <div class="k">Total load</div><div class="v">${fmtPct(cpu.usage)}</div>
        <div class="k">Load avg</div><div class="v">${load.one.toFixed(2)} · ${load.five.toFixed(2)} · ${load.fifteen.toFixed(2)}</div>
      </div>
      <div class="cores">${cores}</div>
    </div>`;
}

function memoryCard(m) {
  return `
    <div class="card">
      <h3>Memory</h3>
      <div class="mem-row">
        ${meter("RAM", m.used, m.total, m.percent, "var(--ink)")}
        ${meter("Swap", m.swap_used, m.swap_total, m.swap_percent, "var(--ink-soft)")}
      </div>
    </div>`;
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

function disksCard(disks) {
  const rows = (disks || [])
    .map(
      (d) => `
      <div class="disk-row">
        <div class="disk-main">
          <div class="disk-mount mono">${esc(d.mount)}</div>
          <div class="disk-meta">${esc(d.fs)} · ${esc(d.kind)} · ${esc(d.name) || "disk"}</div>
        </div>
        <div class="disk-bar"><div class="fill" style="width:${Math.min(100, d.percent)}%;background:${barColor(d.percent)}"></div></div>
        <div class="disk-nums">${fmtBytes(d.used)} / ${fmtBytes(d.total)} <span class="disk-pct">${fmtPct(d.percent)}</span></div>
      </div>`
    )
    .join("") || `<div class="empty">No disks reported.</div>`;
  return `<div class="card"><h3>Storage Devices</h3><div class="disks">${rows}</div></div>`;
}

function barColor(pct) {
  if (pct >= 90) return "var(--red)";
  if (pct >= 75) return "var(--amber)";
  return "var(--green)";
}

function kvCard(title, rows) {
  const body = rows
    .map(([k, v]) => `<div class="k">${esc(k)}</div><div class="v">${esc(v || "—")}</div>`)
    .join("");
  return `<div class="card"><h3>${esc(title)}</h3><div class="kv">${body}</div></div>`;
}
