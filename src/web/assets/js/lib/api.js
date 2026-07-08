// Thin fetch wrapper + typed endpoints for the backend JSON/SSE API.

export async function api(path, opts) {
  const res = await fetch(path, opts);
  const body = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(body.error || `Request failed (${res.status})`);
  return body;
}

export const getConfig = () => api("/api/config");
export const getMetrics = () => api("/api/metrics");
export const getSystem = () => api("/api/system");
export const getResults = () => api("/api/results");

export function startScan(payload) {
  return api("/api/scan", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
}

export function deletePaths(paths) {
  return api("/api/delete", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ paths }),
  });
}

// Toast helper (shared element in index.html).
export function toast(msg, kind) {
  const el = document.getElementById("toast");
  el.textContent = msg;
  el.className = "toast " + (kind || "");
  clearTimeout(el._t);
  el._t = setTimeout(() => el.classList.add("hidden"), 3500);
}
