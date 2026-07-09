//! Public scanner API and scan orchestration.
//!
//! [`Scanner`] owns the scan settings and coordinates the two halves of a scan:
//! a background task that publishes progress and the blocking parallel walk in
//! [`super::walk`]. The heavy directory traversal, path-skipping and
//! classification live in sibling modules to keep this file focused on wiring.

use crate::models::{FileEntry, ScanProgress, ScanResult};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub struct Scanner {
    min_size_bytes: u64,
    #[allow(dead_code)]
    max_depth: usize,
}

impl Scanner {
    pub fn new(min_size_mb: u64, max_depth: usize) -> Self {
        Self {
            min_size_bytes: min_size_mb * 1024 * 1024,
            max_depth,
        }
    }

    /// Full scan - scans EVERYTHING, tracks inodes to avoid double-counting hard links.
    pub async fn scan_with_progress(
        &self,
        path: &Path,
        progress: Arc<Mutex<ScanProgress>>,
    ) -> Result<ScanResult> {
        let min_size = self.min_size_bytes;
        let root = path.to_path_buf();

        // Determine if this is a full disk scan and get root device ID
        let path_str = path.to_string_lossy();
        let is_full_disk =
            path_str == "/" || path_str == "/Users" || path_str.starts_with("/Users/");

        #[cfg(unix)]
        let root_device_id = std::fs::metadata(path).ok().map(|m| m.dev());
        #[cfg(not(unix))]
        let root_device_id: Option<u64> = None;

        // Atomic counters
        let files_count = Arc::new(AtomicUsize::new(0));
        let dirs_count = Arc::new(AtomicUsize::new(0));
        let total_size = Arc::new(AtomicU64::new(0));
        let is_complete = Arc::new(AtomicBool::new(false));
        let scan_started = Arc::new(AtomicBool::new(false));

        let entries = Arc::new(Mutex::new(Vec::<FileEntry>::with_capacity(50000)));
        let current_path = Arc::new(Mutex::new(String::new()));

        // Track category sizes during scan
        let category_sizes = Arc::new(std::sync::Mutex::new(HashMap::<String, u64>::new()));

        // Progress updater task
        let files_count_c = files_count.clone();
        let dirs_count_c = dirs_count.clone();
        let total_size_c = total_size.clone();
        let is_complete_c = is_complete.clone();
        let scan_started_c = scan_started.clone();
        let entries_c = entries.clone();
        let current_path_c = current_path.clone();
        let progress_c = progress.clone();
        let category_sizes_c = category_sizes.clone();

        let progress_task = tokio::spawn(async move {
            loop {
                if !scan_started_c.load(Ordering::Acquire) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    continue;
                }

                if is_complete_c.load(Ordering::Acquire) {
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let mut prog = progress_c.lock().await;
                prog.files_scanned = files_count_c.load(Ordering::Relaxed);
                prog.dirs_scanned = dirs_count_c.load(Ordering::Relaxed);
                prog.total_size_scanned = total_size_c.load(Ordering::Relaxed);

                if let Ok(cp) = current_path_c.try_lock() {
                    prog.current_path = cp.clone();
                }

                if let Ok(ents) = entries_c.try_lock() {
                    let top: Vec<_> = ents.iter().filter(|e| !e.is_dir).take(15).cloned().collect();
                    prog.entries = top;
                }

                // Update category sizes from shared tracker
                if let Ok(cats) = category_sizes_c.try_lock() {
                    prog.category_sizes = cats.clone();
                }
            }
        });

        // Main scan
        let entries_main = entries.clone();
        let current_path_main = current_path.clone();
        let files_count_main = files_count.clone();
        let dirs_count_main = dirs_count.clone();
        let total_size_main = total_size.clone();
        let scan_started_main = scan_started.clone();
        let category_sizes_main = category_sizes.clone();

        let scan_result = tokio::task::spawn_blocking(move || -> Result<()> {
            scan_started_main.store(true, Ordering::Release);

            super::walk::scan_all(
                &root,
                min_size,
                is_full_disk,
                root_device_id,
                &entries_main,
                &current_path_main,
                &files_count_main,
                &dirs_count_main,
                &total_size_main,
                &category_sizes_main,
            );

            Ok(())
        })
        .await;

        match scan_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                is_complete.store(true, Ordering::Release);
                let _ = progress_task.await;
                return Err(e);
            }
            Err(e) => {
                is_complete.store(true, Ordering::Release);
                let _ = progress_task.await;
                return Err(anyhow::anyhow!("Scan task failed: {}", e));
            }
        }

        is_complete.store(true, Ordering::Release);
        let _ = progress_task.await;

        let mut final_entries = entries.lock().await.clone();
        final_entries.sort_unstable_by(|a, b| b.size.cmp(&a.size));

        let total_files = files_count.load(Ordering::Relaxed);
        let total_dirs = dirs_count.load(Ordering::Relaxed);
        let total_sz = total_size.load(Ordering::Relaxed);

        {
            let mut prog = progress.lock().await;
            prog.is_complete = true;
            prog.files_scanned = total_files;
            prog.dirs_scanned = total_dirs;
            prog.total_size_scanned = total_sz;
            prog.entries = final_entries.iter().take(20).cloned().collect();
        }

        Ok(ScanResult { entries: final_entries })
    }

    #[allow(dead_code)]
    pub async fn scan(&self, path: &Path) -> Result<ScanResult> {
        let progress = Arc::new(Mutex::new(ScanProgress::default()));
        self.scan_with_progress(path, progress).await
    }
}
