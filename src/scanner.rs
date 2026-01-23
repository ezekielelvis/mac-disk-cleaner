use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_system: bool,
    pub modified: DateTime<Local>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub entries: Vec<FileEntry>,
    pub total_size: u64,
    pub total_files: usize,
    pub total_dirs: usize,
    pub hidden_count: usize,
    pub system_count: usize,
}

#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub current_path: String,
    pub files_scanned: usize,
    pub dirs_scanned: usize,
    pub total_size_scanned: u64,
    pub is_complete: bool,
    pub entries: Vec<FileEntry>,
}

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            current_path: String::new(),
            files_scanned: 0,
            dirs_scanned: 0,
            total_size_scanned: 0,
            is_complete: false,
            entries: Vec::new(),
        }
    }
}

pub struct Scanner {
    min_size_bytes: u64,
    #[allow(dead_code)]
    max_depth: usize,  // Kept for API compatibility but not used
}

impl Scanner {
    pub fn new(min_size_mb: u64, max_depth: usize) -> Self {
        Self {
            min_size_bytes: min_size_mb * 1024 * 1024,
            max_depth,
        }
    }

    #[inline(always)]
    pub fn is_hidden(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
    }

    #[inline(always)]
    pub fn is_system_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        path_str.starts_with("/System") ||
        path_str.starts_with("/usr") ||
        path_str.starts_with("/bin") ||
        path_str.starts_with("/sbin") ||
        path_str.contains("/Library/Keychains") ||
        path_str.contains("/.ssh") ||
        path_str.contains("/.gnupg") ||
        path_str.ends_with(".zshrc") ||
        path_str.ends_with(".bashrc") ||
        path_str.ends_with(".bash_profile")
    }

    pub fn get_system_warning(path: &Path) -> Option<String> {
        let path_str = path.to_string_lossy();
        
        if path_str.contains("Keychains") {
            return Some("⚠️ Contains encryption keys!".to_string());
        }
        if path_str.contains(".ssh") {
            return Some("⚠️ SSH keys!".to_string());
        }
        if path_str.contains(".gnupg") {
            return Some("⚠️ GPG keys!".to_string());
        }
        if path_str.contains("/System") {
            return Some("🛑 SYSTEM FILES!".to_string());
        }
        None
    }

    /// Full unrestricted scan - scans everything until complete
    pub async fn scan_with_progress(
        &self,
        path: &Path,
        progress: Arc<Mutex<ScanProgress>>,
    ) -> Result<ScanResult> {
        let min_size = self.min_size_bytes;
        let root = path.to_path_buf();
        
        // Atomic counters
        let files_count = Arc::new(AtomicUsize::new(0));
        let dirs_count = Arc::new(AtomicUsize::new(0));
        let total_size = Arc::new(AtomicU64::new(0));
        let is_complete = Arc::new(AtomicBool::new(false));
        
        let entries = Arc::new(Mutex::new(Vec::<FileEntry>::with_capacity(10000)));
        let current_path = Arc::new(Mutex::new(String::new()));
        
        // Progress updater task
        let files_count_c = files_count.clone();
        let dirs_count_c = dirs_count.clone();
        let total_size_c = total_size.clone();
        let is_complete_c = is_complete.clone();
        let entries_c = entries.clone();
        let current_path_c = current_path.clone();
        let progress_c = progress.clone();
        
        let progress_task = tokio::spawn(async move {
            loop {
                if is_complete_c.load(Ordering::Relaxed) {
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
                    let top: Vec<_> = ents.iter()
                        .filter(|e| !e.is_dir)
                        .take(15)
                        .cloned()
                        .collect();
                    prog.entries = top;
                }
            }
        });

        // Main scan in blocking thread
        let entries_main = entries.clone();
        let current_path_main = current_path.clone();
        let files_count_main = files_count.clone();
        let dirs_count_main = dirs_count.clone();
        let total_size_main = total_size.clone();
        
        tokio::task::spawn_blocking(move || {
            Self::scan_all(
                &root,
                min_size,
                &entries_main,
                &current_path_main,
                &files_count_main,
                &dirs_count_main,
                &total_size_main,
            );
        }).await?;
        
        is_complete.store(true, Ordering::SeqCst);
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

        Ok(ScanResult {
            entries: final_entries,
            total_size: total_sz,
            total_files,
            total_dirs,
            hidden_count: 0,
            system_count: 0,
        })
    }

    /// Scan everything - no depth limit, no skipping
    fn scan_all(
        root: &Path,
        min_size: u64,
        entries: &Arc<Mutex<Vec<FileEntry>>>,
        current_path: &Arc<Mutex<String>>,
        files_count: &Arc<AtomicUsize>,
        dirs_count: &Arc<AtomicUsize>,
        total_size: &Arc<AtomicU64>,
    ) {
        // Queue of directories to scan - NO depth tracking, scan everything
        let mut queue: VecDeque<PathBuf> = VecDeque::with_capacity(10000);
        queue.push_back(root.to_path_buf());
        
        let mut batch_entries = Vec::with_capacity(100);
        let mut update_counter = 0u32;
        
        while let Some(dir_path) = queue.pop_front() {
            // Update current path every 100 directories
            update_counter += 1;
            if update_counter % 100 == 0 {
                if let Ok(mut cp) = current_path.try_lock() {
                    *cp = dir_path.to_string_lossy().to_string();
                }
            }
            
            // Try to read directory - if it fails, just skip it and continue
            let read_dir = match std::fs::read_dir(&dir_path) {
                Ok(rd) => rd,
                Err(_) => continue,  // Permission denied, etc - just skip
            };

            for entry in read_dir {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                // Get metadata - if it fails, skip this entry
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                
                let is_dir = metadata.is_dir();
                let size = if is_dir { 0 } else { metadata.len() };
                
                if is_dir {
                    dirs_count.fetch_add(1, Ordering::Relaxed);
                    // Add ALL directories to queue - no filtering
                    queue.push_back(path.clone());
                } else {
                    files_count.fetch_add(1, Ordering::Relaxed);
                    total_size.fetch_add(size, Ordering::Relaxed);
                }

                // Track items above minimum size (or all directories for navigation)
                if size >= min_size || is_dir {
                    let is_hidden = name_str.starts_with('.');
                    let is_system = Self::is_system_path(&path);
                    
                    let modified = metadata.modified()
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .map(|d| Local::now() - chrono::Duration::seconds(d.as_secs() as i64))
                        .unwrap_or_else(Local::now);

                    batch_entries.push(FileEntry {
                        path,
                        size,
                        is_dir,
                        is_hidden,
                        is_system,
                        modified,
                        name: name_str.to_string(),
                    });
                }
            }
            
            // Flush batch to shared storage periodically
            if batch_entries.len() >= 100 {
                if let Ok(mut ents) = entries.try_lock() {
                    ents.append(&mut batch_entries);
                }
                batch_entries.clear();
            }
        }
        
        // Final flush
        if !batch_entries.is_empty() {
            if let Ok(mut ents) = entries.try_lock() {
                ents.append(&mut batch_entries);
            }
        }
    }

    #[allow(dead_code)]
    pub async fn scan(&self, path: &Path) -> Result<ScanResult> {
        let progress = Arc::new(Mutex::new(ScanProgress::default()));
        self.scan_with_progress(path, progress).await
    }
}
