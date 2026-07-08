// Dashboard page — four live Chart.js charts (CPU, memory, disk, network) laid
// out 2x2. The data comes from the app-wide metrics store, so the history keeps
// running while you're on other pages; this page just subscribes to it.
//
/* global Chart */ // provided by the vendored UMD build loaded in index.html

import { subscribeMetrics, CAPACITY } from "../lib/metrics.js";
import { fmtBytes, fmtRate, fmtPct } from "../lib/format.js";

// Per-metric indicator colors — each chart reads as its own signal.
const COLORS = {
  cpu: "#4f46e5",   // indigo
  mem: "#16a34a",   // green
  disk: "#d97706",  // amber
  netRx: "#4f46e5", // indigo (download)
  netTx: "#0891b2", // cyan   (upload)
};
const GRID = "#eeeeee";

export async function mount(view) {
  view.innerHTML = `
    <div class="page-head">
      <h1>Dashboard</h1>
      <div class="sub">Live system &amp; storage metrics, updating every second.</div>
    </div>
    <div class="metric-grid">
      ${card("cpu", "CPU Usage", COLORS.cpu)}
      ${card("mem", "Memory Usage", COLORS.mem)}
      ${card("disk", "Disk Usage", COLORS.disk)}
      ${netCard()}
    </div>
  `;

  const cpuChart = lineChart(view.querySelector("#chart-cpu"), [series(COLORS.cpu, true)], 100);
  const memChart = lineChart(view.querySelector("#chart-mem"), [series(COLORS.mem, true)], 100);
  const diskChart = lineChart(view.querySelector("#chart-disk"), [series(COLORS.disk, true)], 100);
  const netChart = lineChart(
    view.querySelector("#chart-net"),
    [series(COLORS.netRx, true), series(COLORS.netTx, true)],
    null
  );

  const el = (id) => view.querySelector(id);
  const labels = Array.from({ length: CAPACITY }, (_, i) => i);

  function apply(chart, arrays) {
    chart.data.labels = labels.slice(0, arrays[0].length);
    arrays.forEach((arr, i) => { chart.data.datasets[i].data = arr; });
    chart.update("none");
  }

  const unsubscribe = subscribeMetrics((s) => {
    const m = s.latest;
    if (!m) return;

    apply(cpuChart, [s.cpu]);
    el("#val-cpu").textContent = fmtPct(m.cpu);

    apply(memChart, [s.mem]);
    el("#val-mem").textContent = fmtPct(m.mem_percent);
    el("#sub-mem").textContent = `${fmtBytes(m.mem_used)} / ${fmtBytes(m.mem_total)}`;

    apply(diskChart, [s.disk]);
    el("#val-disk").textContent = fmtPct(m.disk_percent);
    el("#sub-disk").textContent = `${fmtBytes(m.disk_used)} / ${fmtBytes(m.disk_total)}`;

    apply(netChart, [s.netRx, s.netTx]);
    el("#val-net").textContent = fmtRate(m.net_rx_rate + m.net_tx_rate);
    el("#net-down").textContent = fmtRate(m.net_rx_rate);
    el("#net-up").textContent = fmtRate(m.net_tx_rate);
  });

  // Leave the polling loop running; only tear down this page's view of it.
  return () => {
    unsubscribe();
    cpuChart.destroy();
    memChart.destroy();
    diskChart.destroy();
    netChart.destroy();
  };
}

// A dataset spec: colored line with a soft matching area fill, no point markers.
function series(color, fill) {
  return {
    data: [],
    borderColor: color,
    backgroundColor: fill ? rgba(color, 0.12) : "transparent",
    borderWidth: 1.75,
    fill,
    tension: 0.3,
    pointRadius: 0,
    pointHitRadius: 0,
  };
}

function lineChart(canvas, datasets, yMax) {
  return new Chart(canvas, {
    type: "line",
    data: { labels: [], datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      interaction: { intersect: false },
      scales: {
        x: { display: false },
        y: {
          display: false,
          min: 0,
          ...(yMax != null ? { max: yMax } : {}),
          grid: { color: GRID, drawTicks: false },
          border: { display: false },
        },
      },
      plugins: { legend: { display: false }, tooltip: { enabled: false } },
    },
  });
}

function card(id, title, color) {
  return `
    <div class="metric-card">
      <div class="metric-top">
        <h3><i class="metric-ind" style="background:${color}"></i>${title}</h3>
        <div class="metric-value" id="val-${id}">—</div>
      </div>
      <div class="metric-sub" id="sub-${id}">&nbsp;</div>
      <div class="chart-wrap"><canvas id="chart-${id}"></canvas></div>
    </div>`;
}

function netCard() {
  return `
    <div class="metric-card">
      <div class="metric-top">
        <h3><i class="metric-ind" style="background:${COLORS.netRx}"></i>Network I/O</h3>
        <div class="metric-value" id="val-net">—</div>
      </div>
      <div class="metric-sub metric-legend">
        <span><i class="dot" style="background:${COLORS.netRx}"></i>DOWN <b id="net-down">—</b></span>
        <span><i class="dot" style="background:${COLORS.netTx}"></i>UP <b id="net-up">—</b></span>
      </div>
      <div class="chart-wrap"><canvas id="chart-net"></canvas></div>
    </div>`;
}

// #rrggbb -> rgba(...) with the given alpha (for soft area fills).
function rgba(hex, a) {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}
