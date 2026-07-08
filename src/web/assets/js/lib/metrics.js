// Global live-metrics store.
//
// Polling is owned by the app shell, not by the dashboard page, so history keeps
// accumulating while you're on another page. The dashboard just subscribes to
// the rolling buffers when it mounts and unsubscribes when it leaves — the timer
// and the data survive navigation.

import { getMetrics } from "./api.js";

export const CAPACITY = 60; // ~60s of history at 1 sample/second

const state = {
  cpu: [],
  mem: [],
  disk: [],
  netRx: [],
  netTx: [],
  latest: null, // most recent raw /api/metrics payload
};

const subscribers = new Set();
let timer = null;

function push(arr, value) {
  arr.push(Number.isFinite(value) ? value : 0);
  if (arr.length > CAPACITY) arr.shift();
}

function record(m) {
  push(state.cpu, m.cpu);
  push(state.mem, m.mem_percent);
  push(state.disk, m.disk_percent);
  push(state.netRx, m.net_rx_rate);
  push(state.netTx, m.net_tx_rate);
  state.latest = m;
  subscribers.forEach((fn) => {
    try { fn(state); } catch (_) {}
  });
}

async function tick() {
  try {
    record(await getMetrics());
  } catch (_) {
    /* transient fetch error — keep the previous history, try again next tick */
  }
}

/** Start the app-wide polling loop. Idempotent: safe to call more than once. */
export function startMetrics(pollMs = 1000) {
  if (timer) return;
  tick();
  timer = setInterval(tick, pollMs);
}

/**
 * Subscribe to metric updates. The callback fires immediately with the current
 * state (if any samples exist) and then on every new sample. Returns an
 * unsubscribe function; unsubscribing never stops the shared polling loop.
 */
export function subscribeMetrics(fn) {
  subscribers.add(fn);
  if (state.latest) fn(state);
  return () => subscribers.delete(fn);
}

export function metricsState() {
  return state;
}
