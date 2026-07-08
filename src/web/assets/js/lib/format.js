// Small formatting helpers shared across pages.

export function fmtBytes(n) {
  if (!n || n < 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
  return (n / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1) + " " + units[i];
}

// Bytes/sec → human rate (e.g. "1.2 MB/s").
export function fmtRate(bytesPerSec) {
  return fmtBytes(bytesPerSec) + "/s";
}

export function fmtPct(n) {
  return (n || 0).toFixed(0) + "%";
}

// Seconds → "3d 4h 12m".
export function fmtUptime(secs) {
  secs = Math.floor(secs || 0);
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const parts = [];
  if (d) parts.push(d + "d");
  if (h) parts.push(h + "h");
  parts.push(m + "m");
  return parts.join(" ");
}

// Unix seconds → local date string.
export function fmtDate(unixSecs) {
  if (!unixSecs) return "—";
  return new Date(unixSecs * 1000).toLocaleString();
}

export function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}
