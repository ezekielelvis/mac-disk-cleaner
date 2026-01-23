use anyhow::Result;
use crate::models::{FileEntry, ScanResult, ScanProgress};
use super::utils::{is_system_path, is_hidden_path};
use super::fs_ops::*;
use super::dir_calculator::DirectorySizeCalculator;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;

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
        
        let mut batch_entries = Vec::with_capacity(500);
        let mut update_counter = 0u32;
        let mut skipped_dirs = 0usize;
        let mut error_count = 0usize;
        let mut skipped_bytes_estimate = 0u64;
        
        // Track directory sizes to calculate recursive sizes
        let mut dir_calculator = DirectorySizeCalculator::new();
        
        while let Some(dir_path) = queue.pop_front() {
            // Update current path every 50 directories
            update_counter += 1;
            if update_counter % 50 == 0 {
                if let Ok(mut cp) = current_path.try_lock() {
                    *cp = dir_path.to_string_lossy().to_string();
                }
            }
            
            // Try to read directory - if it fails, estimate its size
            let entries_in_dir = match try_read_directory(&dir_path) {
                Some(entries) => entries,
                None => {
                    skipped_dirs += 1;
                    
                    // Estimate size of inaccessible directory
                    let estimated_size = estimate_inaccessible_size(&dir_path);
                    if estimated_size > 0 {
                        skipped_bytes_estimate += estimated_size;
                        total_size.fetch_add(estimated_size, Ordering::Relaxed);
                    }
                    
                    // Log every 100th error to avoid spam
                    if skipped_dirs % 100 == 1 {
                        eprintln!("⚠️  Skipped {} directories (~{} inaccessible)", 
                            skipped_dirs,
                            humansize::format_size(skipped_bytes_estimate, humansize::DECIMAL));
                    }
                    continue;
                }
            };

            let mut dir_total_size = 0u64;
            
            for entry in entries_in_dir {
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                // Get metadata - if it fails, skip this entry
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_e) => {
                        error_count += 1;
                        if error_count % 1000 == 1 {
                            eprintln!("⚠️  Encountered {} metadata errors, continuing...", error_count);
                        }
                        continue;
                    }
                };
                
                let is_dir = metadata.is_dir();
                
                // Use disk allocation for accurate sizing
                let size = get_disk_allocation(&metadata, is_dir);
                
                if is_dir {
                    dirs_count.fetch_add(1, Ordering::Relaxed);
                    queue.push_back(path.clone());
                } else {
                    files_count.fetch_add(1, Ordering::Relaxed);
                    total_size.fetch_add(size, Ordering::Relaxed);
                    dir_total_size += size;
                }

                // Track items above minimum size (or all directories for navigation)
                if size >= min_size || is_dir {
                    let is_hidden = is_hidden_path(&path);
                    let is_system = is_system_path(&path);
                    let modified = get_modified_time(&metadata);

                    batch_entries.push(FileEntry {
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
            
            // Store this directory's immediate size
            dir_calculator.record_directory_size(dir_path.clone(), dir_total_size);
            
            // Flush batch to shared storage periodically
            if batch_entries.len() >= 500 {
                let runtime = tokio::runtime::Handle::try_current();
                if let Ok(handle) = runtime {
                    handle.block_on(async {
                        let mut ents = entries.lock().await;
                        ents.append(&mut batch_entries);
                    });
                } else {
                    let mut retries = 0;
                    while retries < 10 {
                        if let Ok(mut ents) = entries.try_lock() {
                            ents.append(&mut batch_entries);
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        retries += 1;
                    }
                    if retries == 10 {
                        eprintln!("⚠️  Warning: Failed to flush {} entries after retries", batch_entries.len());
                    }
                }
                batch_entries.clear();
            }
        }
        
        // Final flush
        if !batch_entries.is_empty() {
            let runtime = tokio::runtime::Handle::try_current();
            if let Ok(handle) = runtime {
                handle.block_on(async {
                    let mut ents = entries.lock().await;
                    ents.append(&mut batch_entries);
                });
            } else {
                let mut retries = 0;
                while retries < 50 && !batch_entries.is_empty() {
                    if let Ok(mut ents) = entries.try_lock() {
                        ents.append(&mut batch_entries);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    retries += 1;
                }
                if !batch_entries.is_empty() {
                    eprintln!("❌ ERROR: Lost {} entries in final flush!", batch_entries.len());
                }
            }
        }
        
        // Calculate recursive directory sizes
        let runtime = tokio::runtime::Handle::try_current();
        if let Ok(handle) = runtime {
            handle.block_on(async {
                let mut ents = entries.lock().await;
                dir_calculator.calculate_recursive_sizes(&mut ents);
            });
        }
        
        // Final status
        let total_scanned = total_size.load(Ordering::Relaxed);
        if skipped_dirs > 0 || error_count > 0 {
            eprintln!("✓ Scan completed:");
            eprintln!("  • Scanned: {}", humansize::format_size(total_scanned, humansize::DECIMAL));
            if skipped_bytes_estimate > 0 {
                eprintln!("  • Inaccessible (estimated): {}", humansize::format_size(skipped_bytes_estimate, humansize::DECIMAL));
                eprintln!("  • Total (estimated): {}", humansize::format_size(total_scanned + skipped_bytes_estimate, humansize::DECIMAL));
            }
            eprintln!("  • Skipped {} directories, {} errors", skipped_dirs, error_count);
        }
    }

    #[allow(dead_code)]
    pub async fn scan(&self, path: &Path) -> Result<ScanResult> {
        let progress = Arc::new(Mutex::new(ScanProgress::default()));
        self.scan_with_progress(path, progress).await
    }
}
