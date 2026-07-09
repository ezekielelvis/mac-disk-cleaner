//! Rules for paths the scanner should not descend into.
//!
//! These exclusions keep the scan from hanging on virtual filesystems and from
//! double-counting the same bytes that macOS exposes under several mount points
//! or firmlinks.

use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Paths to skip to avoid double-counting or hanging.
/// On macOS with APFS, the same files can appear under multiple mount points.
pub(super) const VIRTUAL_PATHS: &[&str] = &[
    "/dev",            // Device files - not real disk usage
    "/proc",           // Process info (Linux)
    "/sys",            // Kernel info (Linux)
    "/.vol",           // macOS volume references (can cause loops)
    "/private/var/vm", // macOS virtual memory - swap files
];

/// Additional paths to skip when scanning root (/) to prevent double-counting.
/// These are mount points or firmlinks that would cause files to be counted twice.
pub(super) const ROOT_SKIP_PATHS: &[&str] = &[
    "/Volumes",             // External drives, network mounts, Time Machine
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
    "/.Spotlight-V100",     // Spotlight index
    "/.fseventsd",          // Filesystem events
    "/.DocumentRevisions-V100", // Document versions
    // "/System/Library/Caches", // System caches - protected
    // "/.Trashes",         // Trash on other volumes
    "/Network", // Network mounts
];

/// Check if path should be skipped (virtual or duplicate mount point).
#[inline(always)]
pub(super) fn should_skip_path(path: &Path, is_full_disk: bool) -> bool {
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

/// Check if path crosses to a different filesystem (different device).
#[cfg(unix)]
#[inline(always)]
pub(super) fn is_different_filesystem(path: &Path, root_device: Option<u64>) -> bool {
    if let Some(root_dev) = root_device {
        if let Ok(metadata) = std::fs::metadata(path) {
            return metadata.dev() != root_dev;
        }
    }
    false
}

#[cfg(not(unix))]
#[inline(always)]
pub(super) fn is_different_filesystem(_path: &Path, _root_device: Option<u64>) -> bool {
    false
}
