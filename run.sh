#!/usr/bin/env bash
# One command to build and launch Disk Cleaner.
#
# The Rust binary is a self-contained web app: it runs the backend service
# (scanner + system monitor + JSON/SSE API) AND serves the frontend from the
# same process, then opens your browser. So "run everything" is just this.
#
# Usage:
#   ./run.sh                 # build (release) + run on http://127.0.0.1:8080
#   ./run.sh --port 9000     # any flags are passed straight through to the app
#   ./run.sh --home          # default the Cleaner scan to your home directory
set -euo pipefail

cd "$(dirname "$0")"

echo "Building Disk Cleaner (release)…"
cargo build --release

echo "Starting service + web UI…"
exec ./target/release/disk-cleaner "$@"
