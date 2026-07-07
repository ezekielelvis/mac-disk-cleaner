// Web UI module — an axum server that serves an embedded single-page app
// and exposes the scanner/analyzer/cleaner over a small JSON + SSE API.
mod dto;
mod server;

pub use server::run_server;
