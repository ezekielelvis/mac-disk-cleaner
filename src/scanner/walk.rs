//! The parallel directory walk that powers a scan.
//!
//! A pool of worker threads pull directories off a shared work queue, read each
//! one, stream matching entries back over a channel and push newly discovered
//! subdirectories back onto the queue. Hard links are de-duplicated by inode so
//! their bytes are only counted once.

use super::{classify, skip};
use crate::models::FileEntry;
use chrono::{DateTime, Local};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Number of parallel worker threads for scanning.
const NUM_WORKERS: usize = 4;

/// Scan everything using parallel workers with inode tracking.
pub(super) fn scan_all(
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

                        process_directory(
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

/// Process a single directory.
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
    if skip::should_skip_path(dir_path, is_full_disk) {
        skipped_virtual.fetch_add(1, Ordering::Relaxed);
        return;
    }

    // Skip paths on different filesystems to avoid counting mounted volumes
    if skip::is_different_filesystem(dir_path, root_device_id) {
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
            if skip::should_skip_path(&path, is_full_disk) {
                skipped_virtual.fetch_add(1, Ordering::Relaxed);
            } else if skip::is_different_filesystem(&path, root_device_id) {
                skipped_different_fs.fetch_add(1, Ordering::Relaxed);
            } else {
                new_dirs.push(path.clone());
            }
        } else if !is_hardlink_dupe {
            files_count.fetch_add(1, Ordering::Relaxed);
            total_size.fetch_add(size, Ordering::Relaxed);

            // Track category sizes
            let is_hidden = classify::is_hidden_path(&path);
            let is_system = classify::is_system_path(&path);
            let category = classify::categorize_path(&path, is_hidden, is_system);
            *local_category_sizes.entry(category.to_string()).or_insert(0) += size;
        }

        // Track items above minimum size or directories
        if (size >= min_size && !is_hardlink_dupe) || is_dir {
            let is_hidden = classify::is_hidden_path(&path);
            let is_system = classify::is_system_path(&path);
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

fn get_modified_time(metadata: &std::fs::Metadata) -> DateTime<Local> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| Local::now() - chrono::Duration::seconds(d.as_secs() as i64))
        .unwrap_or_else(Local::now)
}
