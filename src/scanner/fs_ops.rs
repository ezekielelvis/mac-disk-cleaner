use std::path::Path;
use std::fs::{DirEntry, Metadata};
use chrono::{DateTime, Local};

/// Read a directory and return its entries
/// Returns None if the directory cannot be read
pub fn try_read_directory(dir_path: &Path) -> Option<Vec<DirEntry>> {
    match std::fs::read_dir(dir_path) {
        Ok(read_dir) => {
            let entries: Vec<DirEntry> = read_dir
                .filter_map(|entry_result| entry_result.ok())
                .collect();
            Some(entries)
        }
        Err(_) => None,
    }
}

/// Get metadata for a path with error handling
pub fn try_get_metadata(path: &Path) -> Option<Metadata> {
    std::fs::metadata(path).ok()
}

/// Get the disk space allocation for a file or directory
/// Uses block-level allocation on Unix systems for accurate disk usage
pub fn get_disk_allocation(metadata: &Metadata, is_dir: bool) -> u64 {
    if is_dir {
        return 0; // Directories are calculated recursively
    }
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let blocks = metadata.blocks();
        blocks * 512
    }
    
    #[cfg(not(unix))]
    {
        metadata.len()
    }
}

/// Estimate the size of an inaccessible directory based on its metadata
pub fn estimate_inaccessible_size(dir_path: &Path) -> u64 {
    if let Some(metadata) = try_get_metadata(dir_path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let blocks = metadata.blocks();
            blocks * 512
        }
        #[cfg(not(unix))]
        {
            0
        }
    } else {
        0
    }
}

/// Get the last modified time for a metadata object
pub fn get_modified_time(metadata: &Metadata) -> DateTime<Local> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| Local::now() - chrono::Duration::seconds(d.as_secs() as i64))
        .unwrap_or_else(Local::now)
}
