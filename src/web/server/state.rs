//! Shared application state threaded through every request handler.

use crate::analyzer::FileCategory;
use crate::models::{FileEntry, ScanProgress};
use crate::web::sysmon::SysMonitor;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Categorized scan output kept in memory after a scan completes.
pub(super) struct ResultsState {
    pub(super) scan_path: PathBuf,
    pub(super) entries: Vec<(FileEntry, FileCategory)>,
}

pub(super) struct Inner {
    pub(super) progress: Arc<Mutex<ScanProgress>>,
    pub(super) results: Mutex<Option<ResultsState>>,
    pub(super) scanning: AtomicBool,
    pub(super) monitor: Mutex<SysMonitor>,
    pub(super) default_path: PathBuf,
    pub(super) default_min_size: u64,
    pub(super) default_depth: usize,
}

#[derive(Clone)]
pub struct AppState {
    pub(super) inner: Arc<Inner>,
}
