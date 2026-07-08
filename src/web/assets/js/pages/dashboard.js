// Dashboard page — four live Chart.js charts (CPU, memory, disk, network) laid
// out 2x2. The data comes from the app-wide metrics store, so the history keeps
// running while you're on other pages; this page just subscribes to it.
//
/* global Chart */ // provided by the vendored UMD build loaded in index.html

import { subscribeMetrics, CAPACITY } from "../lib/metrics.js";
import { fmtBytes, fmtRate, fmtPct } from "../lib/format.js";

const INK = "#161616";
const INK_SOFT = "#b8b8b8";
const GRID = "#eeeeee";

export async function mount(view) {
  view.innerHTML = `
    <div class="page-head">
      <h1>Dashboard</h1>
      <div class="sub">Live system &amp; storage metrics, updating every second.</div>
    </div>
    <div class="metric-grid">
      ${card("cpu", "CPU Usage")}
      ${card("mem", "Memory Usage")}
      ${card("disk", "Disk Usage")}
      ${netCard()}
    </div>
  `;

  const cpuChart = lineChart(view.querySelector("#chart-cpu"), [series(INK, true)], 100);
  const memChart = lineChart(view.querySelector("#chart-mem"), [series(INK, true)], 100);
  const diskChart = lineChart(view.querySelector("#chart-disk"), [series(INK, true)], 100);
  const netChart = lineChart(
    view.querySelector("#chart-net"),
    [series(INK, false), series(INK_SOFT, false)],
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

// A dataset spec: flat ink line, optional soft area fill, no point markers.
function series(color, fill) {
  return {
    data: [],
    borderColor: color,
    backgroundColor: fill ? "rgba(22,22,22,0.06)" : "transparent",
    borderWidth: 1.5,
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

function card(id, title) {
  return `
    <div class="metric-card">
      <div class="metric-top">
        <h3>${title}</h3>
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
        <h3>Network I/O</h3>
        <div class="metric-value" id="val-net">—</div>
      </div>
      <div class="metric-sub metric-legend">
        <span><i class="dot" style="background:${INK}"></i>DOWN <b id="net-down">—</b></span>
        <span><i class="dot" style="background:${INK_SOFT}"></i>UP <b id="net-up">—</b></span>
      </div>
      <div class="chart-wrap"><canvas id="chart-net"></canvas></div>
    </div>`;
}
