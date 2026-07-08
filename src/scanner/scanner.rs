use anyhow::Result;
use crate::models::{FileEntry, ScanResult, ScanProgress};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::{VecDeque, HashSet, HashMap};
use tokio::sync::Mutex;
use chrono::{DateTime, Local};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Number of parallel worker threads for scanning
const NUM_WORKERS: usize = 4;

/// Paths to skip to avoid double-counting or hanging
/// On macOS with APFS, the same files can appear under multiple mount points
const VIRTUAL_PATHS: &[&str] = &[
    "/dev",           // Device files - not real disk usage
    "/proc",          // Process info (Linux)
    "/sys",           // Kernel info (Linux)
    "/.vol",          // macOS volume references (can cause loops)
    "/private/var/vm", // macOS virtual memory - swap files
];

/// Additional paths to skip when scanning root (/) to prevent double-counting
/// These are mount points or firmlinks that would cause files to be counted twice
const ROOT_SKIP_PATHS: &[&str] = &[
    "/Volumes",       // External drives, network mounts, Time Machine
    "/System/Volumes/Data", // APFS Data volume (firmlinked to root dirs)
    "/System/Volumes/Preboot",
    "/System/Volumes/Recovery", 
    "/System/Volumes/VM",
    "/System/Volumes/Update",
    "/System/Volumes/Hardware",  
    "/System/Volumes/xarts",
    "/System/Volumes/iSCPreboot",
    "/System/Volumes/Preboot",
    "/private/var/folders", // User temp - often duplicated
    "/.Spotlight-V100",  // Spotlight index
    "/.fseventsd",       // Filesystem events
    "/.DocumentRevisions-V100", // Document versions
    // "/System/Library/Caches", // System caches - protected
    // "/.Trashes",         // Trash on other volumes
    "/Network",          // Network mounts
];

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

    /// Check if path should be skipped (virtual or duplicate mount point)
    #[inline(always)]
    fn should_skip_path(path: &Path, is_full_disk: bool) -> bool {
        let path_str = path.to_string_lossy();
        
        // Always skip virtual paths
        for vp in VIRTUAL_PATHS {
            if path_str.as_ref() == *vp || path_str.starts_with(&format!("{}/", vp)) {
                return true;
            }
        }
        
        // Skip additional paths when doing full disk or home dir scan
        if is_full_disk {
            for skip in ROOT_SKIP_PATHS {
                if path_str.as_ref() == *skip || path_str.starts_with(&format!("{}/", skip)) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check if path crosses to a different filesystem (different device)
    #[cfg(unix)]
    #[inline(always)]
    fn is_different_filesystem(path: &Path, root_device: Option<u64>) -> bool {
        if let Some(root_dev) = root_device {
            if let Ok(metadata) = std::fs::metadata(path) {
                return metadata.dev() != root_dev;
            }
        }
        false
    }
    
    #[cfg(not(unix))]
    #[inline(always)]
    fn is_different_filesystem(_path: &Path, _root_device: Option<u64>) -> bool {
        false
    }

    #[inline(always)]
    fn is_hidden_path(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
    }

    #[inline(always)]
    fn is_system_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        path_str.starts_with("/System") ||
        path_str.starts_with("/usr") ||
        path_str.starts_with("/bin") ||
        path_str.starts_with("/sbin") ||
        path_str.contains("/Library/Keychains") ||
        path_str.contains("/.ssh") ||
        path_str.contains("/.gnupg")
    }
    
    /// Fast path-based categorization for live scanning display
    #[inline(always)]
    fn categorize_path(path: &Path, is_hidden: bool, is_system: bool) -> &'static str {
        let path_str = path.to_string_lossy();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        
        // Check path patterns first
        if path_str.contains("/Caches/") || path_str.contains("/cache/") || name == "Cache" || name == "Caches" {
            return "Cache";
        }
        if path_str.contains("/Temp/") || path_str.contains("/tmp/") || name.starts_with("tmp") {
            return "Temp Files";
        }
        if name == "node_modules" || path_str.contains("/node_modules/") {
            return "node_modules";
        }
        if path_str.contains("/target/") && (path_str.contains("/debug/") || path_str.contains("/release/")) {
            return "Build Artifacts";
        }
        if path_str.contains("/.cargo/") || path_str.contains("/.npm/") || path_str.contains("/.gradle/") ||
           path_str.contains("/CocoaPods/") || path_str.contains("/.m2/") || path_str.contains("/pip/") {
            return "Package Cache";
        }
        if path_str.contains("/Library/") && !is_system {
            return "Library Files";
        }
        if path_str.contains("/Downloads/") {
            return "Downloads";
        }
        if path_str.contains("/Documents/") {
            return "Documents";
        }
        
        // Check by extension
        match ext.as_str() {
            "log" | "logs" => return "Log Files",
            "mp4" | "mkv" | "avi" | "mov" | "mp3" | "wav" | "flac" | "m4a" | "aac" |
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" | "heic" |
            "psd" | "ai" | "svg" => return "Media",
            "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" | "dmg" | "iso" => return "Archives",
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "odt" => return "Documents",
            "o" | "obj" | "class" | "pyc" | "pyo" => return "Build Artifacts",
            _ => {}
        }
        
        // Check flags
        if is_system {
            return "System Files";
        }
        if is_hidden {
            return "Hidden Files";
        }
        
        "Regular"
    }

    /// Full scan - scans EVERYTHING, tracks inodes to avoid double-counting hard links
    pub async fn scan_with_progress(
        &self,
        path: &Path,
        progress: Arc<Mutex<ScanProgress>>,
    ) -> Result<ScanResult> {
        let min_size = self.min_size_bytes;
        let root = path.to_path_buf();
        
        // Determine if this is a full disk scan and get root device ID
        let path_str = path.to_string_lossy();
        let is_full_disk = path_str == "/" || path_str == "/Users" || path_str.starts_with("/Users/");
        
        #[cfg(unix)]
        let root_device_id = std::fs::metadata(path)
            .ok()
            .map(|m| m.dev());
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
                    let top: Vec<_> = ents.iter()
                        .filter(|e| !e.is_dir)
                        .take(15)
                        .cloned()
                        .collect();
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
            
            Self::scan_all(
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
        }).await;
        
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

    /// Scan everything using parallel workers with inode tracking
    fn scan_all(
        root: &Path,
        min_size: u64,
        is_full_disk: bool,
        root_device_id: Option<u64>,
        entries: &Arc<Mutex<Vec<FileEntry>>>,
        current_path: &Arc<Mutex<String>>,
        files_count: &Arc<AtomicUsize>,
        dirs_count: &Arc<AtomicUsize>,
        total_size: &Arc<AtomicU64>,
        category_sizes: &Arc<std::sync::Mutex<HashMap<String, u64>>>,
    ) {
        use std::sync::mpsc;
        use std::thread;
        
        // Work queue
        let work_queue = Arc::new(std::sync::Mutex::new(VecDeque::<PathBuf>::with_capacity(100000)));
        let active_workers = Arc::new(AtomicUsize::new(0));
        let scan_complete = Arc::new(AtomicBool::new(false));
        
        // Track seen inodes to avoid counting hard links multiple times
        let seen_inodes = Arc::new(std::sync::Mutex::new(HashSet::<u64>::with_capacity(500000)));
        
        // Share scan settings with workers
        let is_full_disk_shared = Arc::new(is_full_disk);
        let root_device_shared = Arc::new(root_device_id);
        
        // Counters for logging
        let skipped_permission = Arc::new(AtomicUsize::new(0));
        let skipped_virtual = Arc::new(AtomicUsize::new(0));
        let skipped_different_fs = Arc::new(AtomicUsize::new(0));
        let hardlink_deduped = Arc::new(AtomicUsize::new(0));
        
        let (entry_tx, entry_rx) = mpsc::channel::<Vec<FileEntry>>();
        
        // Initialize queue
        {
            let mut queue = work_queue.lock().unwrap();
            queue.push_back(root.to_path_buf());
        }
        
        // Spawn workers
        let mut handles = Vec::with_capacity(NUM_WORKERS);
        
        for _worker_id in 0..NUM_WORKERS {
            let queue = work_queue.clone();
            let active = active_workers.clone();
            let complete = scan_complete.clone();
            let tx = entry_tx.clone();
            let current_path = current_path.clone();
            let files_count = files_count.clone();
            let dirs_count = dirs_count.clone();
            let total_size = total_size.clone();
            let seen_inodes = seen_inodes.clone();
            let skipped_permission = skipped_permission.clone();
            let skipped_virtual = skipped_virtual.clone();
            let skipped_different_fs = skipped_different_fs.clone();
            let hardlink_deduped = hardlink_deduped.clone();
            let is_full_disk_w = is_full_disk_shared.clone();
            let root_device_w = root_device_shared.clone();
            let category_sizes_w = category_sizes.clone();
            
            let handle = thread::spawn(move || {
                let mut local_entries = Vec::with_capacity(1000);
                let mut update_counter = 0u32;
                let mut local_category_sizes: HashMap<String, u64> = HashMap::new();
                
                loop {
                    let dir_path = {
                        let mut q = queue.lock().unwrap();
                        q.pop_front()
                    };
                    
                    match dir_path {
                        Some(path) => {
                            active.fetch_add(1, Ordering::SeqCst);
                            
                            Self::process_directory(
                                &path,
                                min_size,
                                *is_full_disk_w,
                                *root_device_w,
                                &queue,
                                &mut local_entries,
                                &current_path,
                                &files_count,
                                &dirs_count,
                                &total_size,
                                &seen_inodes,
                                &skipped_permission,
                                &skipped_virtual,
                                &skipped_different_fs,
                                &hardlink_deduped,
                                &mut update_counter,
                                &mut local_category_sizes,
                            );
                            
                            if local_entries.len() >= 1000 {
                                let _ = tx.send(std::mem::take(&mut local_entries));
                                local_entries = Vec::with_capacity(1000);
                            }
                            
                            // Periodically merge local category sizes into shared
                            if update_counter % 100 == 0 && !local_category_sizes.is_empty() {
                                if let Ok(mut shared_cats) = category_sizes_w.try_lock() {
                                    for (cat, size) in local_category_sizes.drain() {
                                        *shared_cats.entry(cat).or_insert(0) += size;
                                    }
                                }
                            }
                            
                            active.fetch_sub(1, Ordering::SeqCst);
                        }
                        None => {
                            if active.load(Ordering::SeqCst) == 0 {
                                let q = queue.lock().unwrap();
                                if q.is_empty() {
                                    break;
                                }
                            }
                            thread::sleep(std::time::Duration::from_micros(100));
                        }
                    }
                    
                    if complete.load(Ordering::Relaxed) {
                        break;
                    }
                }
                
                // Flush remaining local category sizes
                if !local_category_sizes.is_empty() {
                    if let Ok(mut shared_cats) = category_sizes_w.lock() {
                        for (cat, size) in local_category_sizes.drain() {
                            *shared_cats.entry(cat).or_insert(0) += size;
                        }
                    }
                }
                
                if !local_entries.is_empty() {
                    let _ = tx.send(local_entries);
                }
            });
            
            handles.push(handle);
        }
        
        drop(entry_tx);
        
        // Collect entries
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
        
        for handle in handles {
            let _ = handle.join();
        }
        
        scan_complete.store(true, Ordering::Relaxed);
    }
    
    /// Process a single directory
    fn process_directory(
        dir_path: &Path,
        min_size: u64,
        is_full_disk: bool,
        root_device_id: Option<u64>,
        work_queue: &Arc<std::sync::Mutex<VecDeque<PathBuf>>>,
        local_entries: &mut Vec<FileEntry>,
        current_path: &Arc<Mutex<String>>,
        files_count: &Arc<AtomicUsize>,
        dirs_count: &Arc<AtomicUsize>,
        total_size: &Arc<AtomicU64>,
        seen_inodes: &Arc<std::sync::Mutex<HashSet<u64>>>,
        skipped_permission: &Arc<AtomicUsize>,
        skipped_virtual: &Arc<AtomicUsize>,
        skipped_different_fs: &Arc<AtomicUsize>,
        hardlink_deduped: &Arc<AtomicUsize>,
        update_counter: &mut u32,
        local_category_sizes: &mut HashMap<String, u64>,
    ) {
        // Skip virtual paths and mount points that would cause double-counting
        if Self::should_skip_path(dir_path, is_full_disk) {
            skipped_virtual.fetch_add(1, Ordering::Relaxed);
            return;
        }
        
        // Skip paths on different filesystems to avoid counting mounted volumes
        if Self::is_different_filesystem(dir_path, root_device_id) {
            skipped_different_fs.fetch_add(1, Ordering::Relaxed);
            return;
        }
        
        *update_counter += 1;
        if *update_counter % 50 == 0 {
            if let Ok(mut cp) = current_path.try_lock() {
                *cp = dir_path.to_string_lossy().to_string();
            }
        }
        
        // Read directory
        let read_dir = match std::fs::read_dir(dir_path) {
            Ok(rd) => rd,
            Err(_) => {
                skipped_permission.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };
        
        let mut new_dirs = Vec::new();
        
        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            // Get metadata - use symlink_metadata to detect symlinks before following
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            
            let is_dir = metadata.is_dir();
            let is_symlink = metadata.file_type().is_symlink();
            
            // Skip symlinks to avoid double counting and loops
            if is_symlink {
                continue;
            }
            
            // For files, check inode to avoid counting hard links multiple times
            #[cfg(unix)]
            let (size, is_hardlink_dupe) = if !is_dir {
                let inode = metadata.ino();
                let nlink = metadata.nlink();
                
                // If file has multiple hard links, only count it once
                if nlink > 1 {
                    let mut seen = seen_inodes.lock().unwrap();
                    if seen.contains(&inode) {
                        hardlink_deduped.fetch_add(1, Ordering::Relaxed);
                        (0u64, true)
                    } else {
                        seen.insert(inode);
                        (metadata.len(), false)
                    }
                } else {
                    (metadata.len(), false)
                }
            } else {
                (0u64, false)
            };
            
            #[cfg(not(unix))]
            let (size, is_hardlink_dupe) = if !is_dir {
                (metadata.len(), false)
            } else {
                (0u64, false)
            };
            
            if is_dir {
                dirs_count.fetch_add(1, Ordering::Relaxed);
                // Skip directories that would cause double-counting
                if Self::should_skip_path(&path, is_full_disk) {
                    skipped_virtual.fetch_add(1, Ordering::Relaxed);
                } else if Self::is_different_filesystem(&path, root_device_id) {
                    skipped_different_fs.fetch_add(1, Ordering::Relaxed);
                } else {
                    new_dirs.push(path.clone());
                }
            } else if !is_hardlink_dupe {
                files_count.fetch_add(1, Ordering::Relaxed);
                total_size.fetch_add(size, Ordering::Relaxed);
                
                // Track category sizes
                let is_hidden = Self::is_hidden_path(&path);
                let is_system = Self::is_system_path(&path);
                let category = Self::categorize_path(&path, is_hidden, is_system);
                *local_category_sizes.entry(category.to_string()).or_insert(0) += size;
            }
            
            // Track items above minimum size or directories
            if (size >= min_size && !is_hardlink_dupe) || is_dir {
                let is_hidden = Self::is_hidden_path(&path);
                let is_system = Self::is_system_path(&path);
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
        
        // Add new directories to queue
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

fn get_modified_time(metadata: &std::fs::Metadata) -> DateTime<Local> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| Local::now() - chrono::Duration::seconds(d.as_secs() as i64))
        .unwrap_or_else(Local::now)
}
