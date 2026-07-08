// Sidebar navigation. Renders the nav and highlights the active route.

const NAV = [
  { id: "dashboard", label: "Dashboard", desc: "Live metrics" },
  { id: "system", label: "System", desc: "Info & health" },
  { id: "cleaner", label: "Cleaner", desc: "Scan & clean" },
];

import { esc } from "../lib/format.js";

export function renderSidebar(el, activeId, onNavigate) {
  el.innerHTML = `
    <div class="sidebar-brand">
      <div>
        <div class="name">Disk Cleaner</div>
        <div class="tag">System monitor</div>
      </div>
    </div>
    <div class="nav-label">Menu</div>
    ${NAV.map(
      (n) => `
      <button type="button" class="nav-item ${n.id === activeId ? "active" : ""}" data-nav="${n.id}">
        <span>${esc(n.label)}</span>
      </button>`
    ).join("")}
    <div class="sidebar-foot">Running locally · ${esc(location.host)}</div>
  `;

  el.querySelectorAll("[data-nav]").forEach((btn) => {
    btn.addEventListener("click", () => onNavigate(btn.dataset.nav));
  });
}
