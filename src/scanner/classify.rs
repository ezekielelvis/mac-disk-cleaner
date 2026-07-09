//! Fast, path-based classification used while scanning.
//!
//! These helpers only look at the path string, name and extension so they stay
//! cheap enough to run on every file during the live scan. The richer,
//! context-aware categorization happens later in the analyzer.

use std::path::Path;

#[inline(always)]
pub(super) fn is_hidden_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

#[inline(always)]
pub(super) fn is_system_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    path_str.starts_with("/System")
        || path_str.starts_with("/usr")
        || path_str.starts_with("/bin")
        || path_str.starts_with("/sbin")
        || path_str.contains("/Library/Keychains")
        || path_str.contains("/.ssh")
        || path_str.contains("/.gnupg")
}

/// Fast path-based categorization for live scanning display.
#[inline(always)]
pub(super) fn categorize_path(path: &Path, is_hidden: bool, is_system: bool) -> &'static str {
    let path_str = path.to_string_lossy();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

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
    if path_str.contains("/.cargo/") || path_str.contains("/.npm/") || path_str.contains("/.gradle/")
        || path_str.contains("/CocoaPods/") || path_str.contains("/.m2/") || path_str.contains("/pip/")
    {
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
        "mp4" | "mkv" | "avi" | "mov" | "mp3" | "wav" | "flac" | "m4a" | "aac" | "jpg" | "jpeg"
        | "png" | "gif" | "bmp" | "tiff" | "webp" | "heic" | "psd" | "ai" | "svg" => return "Media",
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" | "dmg" | "iso" => return "Archives",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "odt" => {
            return "Documents"
        }
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
