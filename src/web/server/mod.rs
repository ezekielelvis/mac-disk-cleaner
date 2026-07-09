//! Axum server setup: builds the shared state and routing table.
//!
//! Request handling is split by concern — [`state`] holds the shared app state,
//! [`assets`] serves the embedded frontend, and [`handlers`] implements the
//! JSON + SSE API.

mod assets;
mod handlers;
mod state;

use crate::models::ScanProgress;
use crate::web::sysmon::SysMonitor;
use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use state::{AppState, Inner};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run_server(
    default_path: PathBuf,
    min_size: u64,
    depth: usize,
    port: u16,
) -> Result<()> {
    let state = AppState {
        inner: Arc::new(Inner {
            progress: Arc::new(Mutex::new(ScanProgress::default())),
            results: Mutex::new(None),
            scanning: AtomicBool::new(false),
            monitor: Mutex::new(SysMonitor::new()),
            default_path,
            default_min_size: min_size,
            default_depth: depth,
        }),
    };

    let app = Router::new()
        .route("/", get(assets::index))
        .route("/assets/*path", get(assets::serve_asset))
        .route("/api/config", get(handlers::get_config))
        .route("/api/metrics", get(handlers::get_metrics))
        .route("/api/system", get(handlers::get_system))
        .route("/api/scan", post(handlers::post_scan))
        .route("/api/scan/stream", get(handlers::scan_stream))
        .route("/api/results", get(handlers::get_results))
        .route("/api/delete", post(handlers::post_delete))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("\n  Disk Cleaner is running at http://{addr}\n");
    axum::serve(listener, app).await?;
    Ok(())
}
