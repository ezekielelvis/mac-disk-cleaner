# Disk Cleaner 🧹

A cross-platform disk-space analyzer, system monitor, and cleaner. A single Rust
binary runs the **backend service** and serves a clean, minimalist **web UI** —
so one command starts everything.

The interface has three pages behind a sidebar:

- **📊 Dashboard** — four live charts reading straight from your system (CPU,
  memory, disk usage, and network I/O), updating every second.
- **🖥️ System** — operating system, CPU, memory, storage devices, load, and a
  health summary (with temperatures where available).
- **🧹 Cleaner** — scan any path, get a categorized breakdown, and safely reclaim
  space.

Everything is read live via [`sysinfo`](https://crates.io/crates/sysinfo), which
works on **macOS, Linux, and Windows**.

## Quick start ▶️

One command builds and runs the whole thing (backend **and** frontend), then
opens your browser:

```bash
./run.sh
```

That's it. The app is now at `http://127.0.0.1:8080`.

Prefer to use Cargo directly? These are equivalent:

```bash
cargo run --release            # build + run everything
cargo run --release -- --port 9000 --home
```

> The frontend is embedded in and served by the same process — there is no
> separate frontend server to start, no `npm install`, and no build step for the
> UI.

### Prerequisites

- Rust 1.85+ / Cargo (edition 2024)

### Install as a command

```bash
cargo install --path .
disk-cleaner            # run from anywhere
```

## Command-line options ⚙️

All flags are passed through by `./run.sh` as well.

| Flag                | Description                                        | Default            |
| ------------------- | -------------------------------------------------- | ------------------ |
| `--port <PORT>`     | Port for the web UI                                | `8080`             |
| `--path <PATH>`     | Default path pre-filled in the Cleaner             | `/` (full disk)    |
| `--home`            | Default the Cleaner scan to your home directory    | off                |
| `--min-size <MB>`   | Minimum file size to display                       | `1`                |
| `--depth <N>`       | Scan depth limit (`0` = unlimited)                 | `0`                |
| `--no-open`         | Don't open the browser automatically               | off                |

Examples:

```bash
./run.sh --port 9000            # run on a different port
./run.sh --home                 # cleaner defaults to ~/
./run.sh --no-open              # headless / remote use
```

## The pages 🧭

### 📊 Dashboard — live metrics

Four time-series charts polling the backend once per second:

1. **CPU Usage** — total load across all cores (%)
2. **Memory Usage** — RAM in use (%) with absolute figures
3. **Disk Usage** — used space on the primary volume (%)
4. **Network I/O** — download/upload throughput

### 🖥️ System — information & health

A refreshed-every-few-seconds snapshot of:

- **OS**: name, version, kernel, architecture, hostname, uptime, boot time
- **Processor**: model, physical/logical cores, frequency, per-core load, load average
- **Memory**: RAM and swap usage
- **Storage devices**: every mounted disk with filesystem, type, and usage
- **Health**: a coarse status (healthy / needs attention / critical) derived from
  memory pressure, swap usage, and temperature, plus per-sensor readings

### 🧹 Cleaner — scan & reclaim

1. Pick a scan: **Full Disk**, **Home Directory**, or a **Custom Path**.
2. Review the results: category breakdown, largest directories, largest files,
   and smart recommendations.
3. Select files (or "Select safe") and delete them. Deletion is guarded so it can
   only remove paths inside the scanned root, and always asks for confirmation.

Smart categorization covers caches, temp files, logs, build artifacts,
`node_modules`, package caches, large/old files, media, archives, and more.
Safety indicators mark system (⚙️) and hidden (◌) files.

## Architecture 🏗️

```text
src/
├── main.rs              # CLI args, launches the server, opens the browser
├── scanner/             # parallel filesystem traversal + progress
├── analyzer/            # categorization + recommendations
├── cleaner/             # safe deletion
├── models/              # shared data types (FileEntry, StorageInfo, …)
└── web/
    ├── server.rs        # axum routes: assets, config, metrics, scan, delete
    ├── sysmon.rs        # cross-platform system monitor (CPU/mem/disk/net/health)
    ├── dto.rs           # JSON payloads for the cleaner
    └── assets/          # the frontend, embedded at compile time
        ├── index.html
        ├── css/         # base.css + one stylesheet per page (plain, non-modular)
        └── js/
            ├── app.js           # hash router + page swapping
            ├── lib/             # api.js, format.js, charts.js (canvas charts)
            ├── components/      # sidebar.js
            └── pages/           # dashboard.js, system.js, cleaner.js
```

**Frontend layout:** the UI is broken into components and pages with a sidebar
router. Styling is deliberately **separated, not modular** — plain global
stylesheets, one per page, on a white background with a clean type scale. No
build tooling: ES modules and CSS are served directly by the Rust backend.

### HTTP API

| Method | Route                | Purpose                                  |
| ------ | -------------------- | ---------------------------------------- |
| GET    | `/`                  | The web UI                               |
| GET    | `/assets/*`          | Embedded CSS/JS assets                    |
| GET    | `/api/config`        | Defaults + current storage               |
| GET    | `/api/metrics`       | Live CPU/mem/disk/network sample         |
| GET    | `/api/system`        | Full system info + health                |
| POST   | `/api/scan`          | Start a scan                             |
| GET    | `/api/scan/stream`   | Server-Sent Events scan progress         |
| GET    | `/api/results`       | Categorized scan results                 |
| POST   | `/api/delete`        | Delete selected paths (root-guarded)     |

## Dependencies 📦

- `axum` + `tokio` — web server and async runtime
- `sysinfo` — cross-platform system metrics
- `walkdir` — directory traversal
- `clap` — command-line arguments
- `serde` / `serde_json` — serialization
- `chrono`, `humansize`, `dirs`, `libc` — dates, sizes, paths, storage stats

## Safety warning ⚠️

The Cleaner permanently deletes files. Always review selected items before
confirming. Deletion is restricted to paths inside the scanned root, but you are
responsible for what you remove — the authors are not liable for data loss.

## License 📄

MIT.
