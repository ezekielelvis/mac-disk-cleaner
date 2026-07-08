// App entry point: hash-based router that swaps pages into <main> and keeps
// the sidebar in sync. Each page module exports `mount(view)` and returns an
// optional cleanup function that the router calls before navigating away.

import { renderSidebar } from "./components/sidebar.js";
import { startMetrics } from "./lib/metrics.js";
import { mount as mountDashboard } from "./pages/dashboard.js";
import { mount as mountSystem } from "./pages/system.js";
import { mount as mountCleaner } from "./pages/cleaner.js";

const ROUTES = {
  dashboard: mountDashboard,
  system: mountSystem,
  cleaner: mountCleaner,
};
const DEFAULT = "dashboard";

const sidebarEl = document.getElementById("sidebar");
const viewEl = document.getElementById("view");
let cleanup = null;

function currentRoute() {
  const id = location.hash.replace(/^#\/?/, "");
  return ROUTES[id] ? id : DEFAULT;
}

function navigate(id) {
  if (location.hash === `#/${id}`) render();
  else location.hash = `#/${id}`;
}

async function render() {
  const id = currentRoute();
  if (typeof cleanup === "function") {
    try { cleanup(); } catch (_) {}
    cleanup = null;
  }
  renderSidebar(sidebarEl, id, navigate);
  viewEl.innerHTML = "";
  try {
    cleanup = await ROUTES[id](viewEl);
  } catch (e) {
    viewEl.innerHTML = `<div class="empty">Failed to load page: ${e.message}</div>`;
  }
}

// Poll metrics for the whole app lifetime so history persists across pages.
startMetrics();

window.addEventListener("hashchange", render);
render();
