use anyhow::Result;
use crate::models::{FileEntry, ScanResult, ScanProgress};
use super::utils::{is_system_path, is_hidden_path, should_skip_path};
use super::fs_ops::*;
use super::dir_calculator::DirectorySizeCalculator;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;

/// Number of parallel worker threads for scanning
const NUM_WORKERS: usize = 4;

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
        let scan_started = Arc::new(AtomicBool::new(false));
        
        let entries = Arc::new(Mutex::new(Vec::<FileEntry>::with_capacity(10000)));
        let current_path = Arc::new(Mutex::new(String::new()));
        
        // Progress updater task
        let files_count_c = files_count.clone();
        let dirs_count_c = dirs_count.clone();
        let total_size_c = total_size.clone();
        let is_complete_c = is_complete.clone();
        let scan_started_c = scan_started.clone();
        let entries_c = entries.clone();
        let current_path_c = current_path.clone();
        let progress_c = progress.clone();
        
        let progress_task = tokio::spawn(async move {
            loop {
                // Wait for scan to start before checking completion
                if !scan_started_c.load(Ordering::Acquire) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    continue;
                }
                
                if is_complete_c.load(Ordering::Acquire) {
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                // Use blocking lock to ensure we don't lose updates
                let mut prog = progress_c.lock().await;
                prog.files_scanned = files_count_c.load(Ordering::Relaxed);
                prog.dirs_scanned = dirs_count_c.load(Ordering::Relaxed);
                prog.total_size_scanned = total_size_c.load(Ordering::Relaxed);
                
                // Try to get current path without blocking too long
                if let Ok(cp) = current_path_c.try_lock() {
                    prog.current_path = cp.clone();
                }
                
                // Try to get top entries without blocking too long
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
        let scan_started_main = scan_started.clone();
        
        let scan_result = tokio::task::spawn_blocking(move || -> Result<()> {
            // Mark scan as started
            scan_started_main.store(true, Ordering::Release);
            
            Self::scan_all(
                &root,
                min_size,
                &entries_main,
                &current_path_main,
                &files_count_main,
                &dirs_count_main,
                &total_size_main,
            );
            
            Ok(())
        }).await;
        
        // Check for scan errors
        match scan_result {
            Ok(Ok(())) => {
                // Scan completed successfully
            }
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
        
        // Mark as complete and wait for progress task
        is_complete.store(true, Ordering::Release);
        let _ = progress_task.await;
        
        // Get final results with blocking lock
        let mut final_entries = entries.lock().await.clone();
        final_entries.sort_unstable_by(|a, b| b.size.cmp(&a.size));
        
        let total_files = files_count.load(Ordering::Relaxed);
        let total_dirs = dirs_count.load(Ordering::Relaxed);
        let total_sz = total_size.load(Ordering::Relaxed);
        
        // Count hidden and system files
        let hidden_count = final_entries.iter().filter(|e| e.is_hidden).count();
        let system_count = final_entries.iter().filter(|e| e.is_system).count();
        
        // Update final progress
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
            hidden_count,
            system_count,
        })
    }

    /// Scan everything using parallel workers - no depth limit
    fn scan_all(
        root: &Path,
        min_size: u64,
        entries: &Arc<Mutex<Vec<FileEntry>>>,
        current_path: &Arc<Mutex<String>>,
        files_count: &Arc<AtomicUsize>,
        dirs_count: &Arc<AtomicUsize>,
        total_size: &Arc<AtomicU64>,
    ) {
        use std::sync::mpsc;
        use std::thread;
        
        // Shared work queue for parallel scanning
        let work_queue = Arc::new(std::sync::Mutex::new(VecDeque::<PathBuf>::with_capacity(50000)));
        let active_workers = Arc::new(AtomicUsize::new(0));
        let scan_complete = Arc::new(AtomicBool::new(false));
        
        // Shared counters for logging
        let skipped_dirs = Arc::new(AtomicUsize::new(0));
        let skipped_virtual = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let skipped_bytes = Arc::new(AtomicU64::new(0));
        
        // Channel for collecting entries from workers
        let (entry_tx, entry_rx) = mpsc::channel::<Vec<FileEntry>>();
        
        // Initialize work queue with starting directories
        {
            let mut queue = work_queue.lock().unwrap();
            
            if root == Path::new("/") {
                // For root scan, add top-level directories in priority order
                let priority_paths = [
                    "/Users",
                    "/Applications", 
                    "/Library",
                    "/System",
                    "/private",
                    "/opt",
                    "/usr",
                ];
                
                for path in priority_paths {
                    let p = PathBuf::from(path);
                    if p.exists() && !should_skip_path(&p) {
                        queue.push_back(p);
                    }
                }
                
                // Add remaining top-level directories
                if let Some(entries_in_root) = try_read_directory(root) {
                    for entry in entries_in_root {
                        let path = entry.path();
                        if path.is_dir() && !should_skip_path(&path) {
                            let path_str = path.to_string_lossy();
                            if !priority_paths.iter().any(|p| path_str == *p) {
                                queue.push_back(path);
                            }
                        }
                    }
                }
            } else {
                queue.push_back(root.to_path_buf());
            }
        }
        
        // Spawn worker threads
        let mut handles = Vec::with_capacity(NUM_WORKERS);
        
        for worker_id in 0..NUM_WORKERS {
            let queue = work_queue.clone();
            let active = active_workers.clone();
            let complete = scan_complete.clone();
            let tx = entry_tx.clone();
            let current_path = current_path.clone();
            let files_count = files_count.clone();
            let dirs_count = dirs_count.clone();
            let total_size = total_size.clone();
            let skipped_dirs = skipped_dirs.clone();
            let skipped_virtual = skipped_virtual.clone();
            let error_count = error_count.clone();
            let skipped_bytes = skipped_bytes.clone();
            
            let handle = thread::spawn(move || {
                let mut local_entries = Vec::with_capacity(500);
                let mut update_counter = 0u32;
                
                loop {
                    // Try to get work
                    let dir_path = {
                        let mut q = queue.lock().unwrap();
                        q.pop_front()
                    };
                    
                    match dir_path {
                        Some(path) => {
                            active.fetch_add(1, Ordering::SeqCst);
                            
                            // Process this directory
                            Self::process_directory(
                                &path,
                                min_size,
                                &queue,
                                &mut local_entries,
                                &current_path,
                                &files_count,
                                &dirs_count,
                                &total_size,
                                &skipped_dirs,
                                &skipped_virtual,
                                &error_count,
                                &skipped_bytes,
                                &mut update_counter,
                                worker_id,
                            );
                            
                            // Flush entries periodically
                            if local_entries.len() >= 500 {
                                let _ = tx.send(std::mem::take(&mut local_entries));
                                local_entries = Vec::with_capacity(500);
                            }
                            
                            active.fetch_sub(1, Ordering::SeqCst);
                        }
                        None => {
                            // No work available - check if we should exit
                            if active.load(Ordering::SeqCst) == 0 {
                                // Double-check the queue is really empty
                                let q = queue.lock().unwrap();
                                if q.is_empty() {
                                    break;
                                }
                            }
                            // Small sleep to avoid busy-waiting
                            thread::sleep(std::time::Duration::from_micros(100));
                        }
                    }
                    
                    if complete.load(Ordering::Relaxed) {
                        break;
                    }
                }
                
                // Final flush of any remaining entries
                if !local_entries.is_empty() {
                    let _ = tx.send(local_entries);
                }
            });
            
            handles.push(handle);
        }
        
        // Drop our sender so the receiver knows when all workers are done
        drop(entry_tx);
        
        // Collect all entries from workers
        let dir_calculator = DirectorySizeCalculator::new();
        let runtime = tokio::runtime::Handle::try_current();
        
        for batch in entry_rx {
            if let Ok(handle) = &runtime {
                handle.block_on(async {
                    let mut ents = entries.lock().await;
                    ents.extend(batch);
                });
            } else {
                loop {
                    if let Ok(mut ents) = entries.try_lock() {
                        ents.extend(batch);
                        break;
                    }
                    thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
        
        // Wait for all workers to complete
        for handle in handles {
            let _ = handle.join();
        }
        
        scan_complete.store(true, Ordering::Relaxed);
        
        // Calculate recursive directory sizes
        if let Ok(handle) = runtime {
            handle.block_on(async {
                let mut ents = entries.lock().await;
                dir_calculator.calculate_recursive_sizes(&mut ents);
            });
        }
        
        // Final status
        let total_scanned = total_size.load(Ordering::Relaxed);
        let skipped_d = skipped_dirs.load(Ordering::Relaxed);
        let skipped_v = skipped_virtual.load(Ordering::Relaxed);
        let errors = error_count.load(Ordering::Relaxed);
        let skipped_b = skipped_bytes.load(Ordering::Relaxed);
        
        if skipped_d > 0 || errors > 0 || skipped_v > 0 {
            eprintln!("✓ Scan completed ({} workers):", NUM_WORKERS);
            eprintln!("  • Scanned: {}", humansize::format_size(total_scanned, humansize::DECIMAL));
            if skipped_b > 0 {
                eprintln!("  • Inaccessible (estimated): {}", humansize::format_size(skipped_b, humansize::DECIMAL));
                eprintln!("  • Total (estimated): {}", humansize::format_size(total_scanned + skipped_b, humansize::DECIMAL));
            }
            eprintln!("  • Skipped {} inaccessible, {} virtual paths, {} errors", skipped_d, skipped_v, errors);
        }
    }
    
    /// Process a single directory - called by worker threads
    fn process_directory(
        dir_path: &Path,
        min_size: u64,
        work_queue: &Arc<std::sync::Mutex<VecDeque<PathBuf>>>,
        local_entries: &mut Vec<FileEntry>,
        current_path: &Arc<Mutex<String>>,
        files_count: &Arc<AtomicUsize>,
        dirs_count: &Arc<AtomicUsize>,
        total_size: &Arc<AtomicU64>,
        skipped_dirs: &Arc<AtomicUsize>,
        skipped_virtual: &Arc<AtomicUsize>,
        error_count: &Arc<AtomicUsize>,
        skipped_bytes: &Arc<AtomicU64>,
        update_counter: &mut u32,
        _worker_id: usize,
    ) {
        // Skip virtual filesystems
        if should_skip_path(dir_path) {
            skipped_virtual.fetch_add(1, Ordering::Relaxed);
            return;
        }
        
        // Update current path periodically
        *update_counter += 1;
        if *update_counter % 100 == 0 {
            if let Ok(mut cp) = current_path.try_lock() {
                *cp = dir_path.to_string_lossy().to_string();
            }
        }
        
        // Try to read directory
        let entries_in_dir = match try_read_directory(dir_path) {
            Some(e) => e,
            None => {
                let count = skipped_dirs.fetch_add(1, Ordering::Relaxed);
                let estimated_size = estimate_inaccessible_size(dir_path);
                if estimated_size > 0 {
                    skipped_bytes.fetch_add(estimated_size, Ordering::Relaxed);
                    total_size.fetch_add(estimated_size, Ordering::Relaxed);
                }
                if count % 500 == 0 {
                    let skipped_b = skipped_bytes.load(Ordering::Relaxed);
                    eprintln!("⚠️  Skipped {} directories (~{} inaccessible)", 
                        count + 1,
                        humansize::format_size(skipped_b, humansize::DECIMAL));
                }
                return;
            }
        };
        
        let mut new_dirs = Vec::new();
        
        for entry in entries_in_dir {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            // Get metadata
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => {
                    let count = error_count.fetch_add(1, Ordering::Relaxed);
                    if count % 1000 == 0 {
                        eprintln!("⚠️  Encountered {} metadata errors, continuing...", count + 1);
                    }
                    continue;
                }
            };
            
            let is_dir = metadata.is_dir();
            let size = get_disk_allocation(&metadata, is_dir);
            
            if is_dir {
                dirs_count.fetch_add(1, Ordering::Relaxed);
                if !should_skip_path(&path) {
                    new_dirs.push(path.clone());
                } else {
                    skipped_virtual.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                files_count.fetch_add(1, Ordering::Relaxed);
                total_size.fetch_add(size, Ordering::Relaxed);
            }
            
            // Track items above minimum size or directories
            if size >= min_size || is_dir {
                let is_hidden = is_hidden_path(&path);
                let is_system = is_system_path(&path);
                let modified = get_modified_time(&metadata);
                
                local_entries.push(FileEntry {
                    path: path.clone(),
                    size,
                    is_dir,
                    is_hidden,
                    is_system,
                    modified,
                    name: name_str.to_string(),
                });
            }
        }
        
        // Add new directories to the work queue
        if !new_dirs.is_empty() {
            let mut queue = work_queue.lock().unwrap();
            for dir in new_dirs {
                queue.push_back(dir);
            }
        }
    }

    #[allow(dead_code)]
    pub async fn scan(&self, path: &Path) -> Result<ScanResult> {
        let progress = Arc::new(Mutex::new(ScanProgress::default()));
        self.scan_with_progress(path, progress).await
    }
}
