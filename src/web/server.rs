use super::dto::*;
use crate::analyzer::{Analyzer, FileCategory};
use crate::cleaner::Cleaner;
use crate::models::{FileEntry, ScanProgress, StorageInfo};
use crate::scanner::Scanner;
use anyhow::Result;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        Html, IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use serde_json::json;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Categorized scan output kept in memory after a scan completes.
struct ResultsState {
    scan_path: PathBuf,
    entries: Vec<(FileEntry, FileCategory)>,
}

struct Inner {
    progress: Arc<Mutex<ScanProgress>>,
    results: Mutex<Option<ResultsState>>,
    scanning: AtomicBool,
    default_path: PathBuf,
    default_min_size: u64,
    default_depth: usize,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

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
            default_path,
            default_min_size: min_size,
            default_depth: depth,
        }),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/style.css", get(style_css))
        .route("/api/config", get(get_config))
        .route("/api/scan", post(post_scan))
        .route("/api/scan/stream", get(scan_stream))
        .route("/api/results", get(get_results))
        .route("/api/delete", post(post_delete))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("\n  🧹 Disk Cleaner is running at http://{addr}\n");
    axum::serve(listener, app).await?;
    Ok(())
}

// ---- static assets ----

async fn index() -> Html<&'static str> {
    Html(include_str!("assets/index.html"))
}

async fn app_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        include_str!("assets/app.js"),
    )
}

async fn style_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("assets/style.css"),
    )
}

// ---- config ----

async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let inner = &state.inner;
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let storage = StorageInfo::from_path(&inner.default_path);
    Json(json!({
        "default_path": inner.default_path.to_string_lossy(),
        "home_path": home.to_string_lossy(),
        "root_path": "/",
        "min_size_mb": inner.default_min_size,
        "max_depth": inner.default_depth,
        "storage": StorageDto::from(&storage),
    }))
}

// ---- scan ----

async fn post_scan(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> Response {
    let path = PathBuf::from(&req.path);
    if !path.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Path does not exist: {}", req.path) })),
        )
            .into_response();
    }
    if state.inner.scanning.swap(true, Ordering::SeqCst) {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "A scan is already running" })),
        )
            .into_response();
    }

    // Reset shared progress for the new scan.
    {
        let mut prog = state.inner.progress.lock().await;
        *prog = ScanProgress::default();
    }
    {
        let mut results = state.inner.results.lock().await;
        *results = None;
    }

    let inner = state.inner.clone();
    let scan_path = path.clone();
    let min_size = req.min_size_mb;
    let depth = req.max_depth;

    tokio::spawn(async move {
        let scanner = Scanner::new(min_size, depth);
        let progress = inner.progress.clone();
        let scan_result = scanner.scan_with_progress(&scan_path, progress).await;

        if let Ok(result) = scan_result {
            // Categorize the whole batch in O(n) (duplicate-name detection
            // included) rather than O(n^2) per-entry context scans.
            let categorized = Analyzer::categorize_all(result.entries);
            let mut results = inner.results.lock().await;
            *results = Some(ResultsState {
                scan_path: scan_path.clone(),
                entries: categorized,
            });
        }

        // Signals completion to the SSE stream: cleared last, once results
        // (if any) are stored, so `complete` never precedes a readable result.
        inner.scanning.store(false, Ordering::SeqCst);
    });

    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}

async fn scan_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let inner = state.inner.clone();
    let stream = async_stream::stream! {
        loop {
            // Completion is tied to the scan task finishing (which flips
            // `scanning` false only *after* results are stored), so a client
            // that fetches /api/results on `complete` never races an empty
            // store.
            let done = !inner.scanning.load(Ordering::SeqCst);
            let dto = {
                let prog = inner.progress.lock().await;
                ProgressDto {
                    files: prog.files_scanned,
                    dirs: prog.dirs_scanned,
                    size: prog.total_size_scanned,
                    current_path: prog.current_path.clone(),
                    complete: done,
                }
            };
            let data = serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string());
            yield Ok(Event::default().data(data));
            if done {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ---- results ----

async fn get_results(State(state): State<AppState>) -> Response {
    let results = state.inner.results.lock().await;
    match results.as_ref() {
        Some(rs) => Json(build_results(&rs.scan_path, &rs.entries)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No scan results available" })),
        )
            .into_response(),
    }
}

// ---- delete ----

async fn post_delete(
    State(state): State<AppState>,
    Json(req): Json<DeleteRequest>,
) -> Response {
    let mut results = state.inner.results.lock().await;
    let Some(rs) = results.as_mut() else {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "No scan results to delete from" })),
        )
            .into_response();
    };

    let scan_root = rs.scan_path.clone();
    // Resolve the root once so the per-path guard compares against a canonical,
    // symlink-free absolute path.
    let canonical_root = scan_root.canonicalize().unwrap_or_else(|_| scan_root.clone());
    let mut delete_results = Vec::new();
    let mut freed = 0u64;
    let mut deleted_paths: Vec<PathBuf> = Vec::new();

    for raw in &req.paths {
        let path = PathBuf::from(raw);
        // Safety: only delete paths that genuinely resolve inside the scanned
        // root. This blocks `..` traversal and symlinked parent directories.
        if !path_within_root(&path, &canonical_root) {
            delete_results.push(DeleteResult { path: raw.clone(), success: false });
            continue;
        }
        // Estimate freed space from what we scanned (FS is gone after delete).
        let size = estimate_from_entries(&rs.entries, &path);
        match Cleaner::delete_file(&path) {
            Ok(_) => {
                freed += size;
                deleted_paths.push(path);
                delete_results.push(DeleteResult { path: raw.clone(), success: true });
            }
            Err(_) => {
                delete_results.push(DeleteResult { path: raw.clone(), success: false });
            }
        }
    }

    // Drop deleted entries so subsequent /api/results reflect the deletion.
    if !deleted_paths.is_empty() {
        rs.entries
            .retain(|(e, _)| !deleted_paths.iter().any(|d| e.path.starts_with(d)));
    }

    let deleted = delete_results.iter().filter(|r| r.success).count();
    Json(DeleteResponse {
        results: delete_results,
        freed,
        deleted,
    })
    .into_response()
}

/// Returns true only if `path` resolves to a location inside `canonical_root`.
///
/// The parent directory is canonicalized (resolving `..` and any symlinked
/// directories) and the final component re-attached, so the final component is
/// never itself dereferenced — a symlink *inside* the root can be removed
/// (only the link, never its outside target), while `../../etc/passwd` and
/// symlinked parents that escape the root are rejected. Fails closed if the
/// parent cannot be resolved.
fn path_within_root(path: &Path, canonical_root: &Path) -> bool {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return false;
    };
    match parent.canonicalize() {
        Ok(canonical_parent) => canonical_parent.join(name).starts_with(canonical_root),
        Err(_) => false,
    }
}

/// Sum the sizes of scanned files at or under `target`.
fn estimate_from_entries(entries: &[(FileEntry, FileCategory)], target: &Path) -> u64 {
    entries
        .iter()
        .filter(|(e, _)| !e.is_dir && e.path.starts_with(target))
        .map(|(e, _)| e.size)
        .sum()
}
