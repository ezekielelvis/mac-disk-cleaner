use crate::models::ScanProgress;
use std::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::models::FileEntry;

pub struct ProgressTracker {
    files_count: Arc<AtomicUsize>,
    dirs_count: Arc<AtomicUsize>,
    total_size: Arc<AtomicU64>,
    is_complete: Arc<AtomicBool>,
    scan_started: Arc<AtomicBool>,
    entries: Arc<Mutex<Vec<FileEntry>>>,
    current_path: Arc<Mutex<String>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            files_count: Arc::new(AtomicUsize::new(0)),
            dirs_count: Arc::new(AtomicUsize::new(0)),
            total_size: Arc::new(AtomicU64::new(0)),
            is_complete: Arc::new(AtomicBool::new(false)),
            scan_started: Arc::new(AtomicBool::new(false)),
            entries: Arc::new(Mutex::new(Vec::with_capacity(10000))),
            current_path: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn mark_started(&self) {
        self.scan_started.store(true, Ordering::Release);
    }

    pub fn mark_complete(&self) {
        self.is_complete.store(true, Ordering::Release);
    }

    pub fn files_count(&self) -> &Arc<AtomicUsize> {
        &self.files_count
    }

    pub fn dirs_count(&self) -> &Arc<AtomicUsize> {
        &self.dirs_count
    }

    pub fn total_size(&self) -> &Arc<AtomicU64> {
        &self.total_size
    }

    pub fn entries(&self) -> &Arc<Mutex<Vec<FileEntry>>> {
        &self.entries
    }

    pub fn current_path(&self) -> &Arc<Mutex<String>> {
        &self.current_path
    }

    pub fn spawn_progress_updater(
        &self,
        progress: Arc<Mutex<ScanProgress>>,
    ) -> tokio::task::JoinHandle<()> {
        let files_count_c = self.files_count.clone();
        let dirs_count_c = self.dirs_count.clone();
        let total_size_c = self.total_size.clone();
        let is_complete_c = self.is_complete.clone();
        let scan_started_c = self.scan_started.clone();
        let entries_c = self.entries.clone();
        let current_path_c = self.current_path.clone();

        tokio::spawn(async move {
            loop {
                if !scan_started_c.load(Ordering::Acquire) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    continue;
                }
                
                if is_complete_c.load(Ordering::Acquire) {
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                let mut prog = progress.lock().await;
                prog.files_scanned = files_count_c.load(Ordering::Relaxed);
                prog.dirs_scanned = dirs_count_c.load(Ordering::Relaxed);
                prog.total_size_scanned = total_size_c.load(Ordering::Relaxed);
                
                if let Ok(cp) = current_path_c.try_lock() {
                    prog.current_path = cp.clone();
                }
                
                if let Ok(ents) = entries_c.try_lock() {
                    let top: Vec<_> = ents.iter()
                        .filter(|e| !e.is_dir)
                        .take(15)
                        .cloned()
                        .collect();
                    prog.entries = top;
                }
            }
        })
    }

    pub fn get_stats(&self) -> (usize, usize, u64) {
        (
            self.files_count.load(Ordering::Relaxed),
            self.dirs_count.load(Ordering::Relaxed),
            self.total_size.load(Ordering::Relaxed),
        )
    }
}
