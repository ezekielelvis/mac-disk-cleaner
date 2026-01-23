use crate::models::FileEntry;
use super::category::FileCategory;

pub struct ClassificationRules;

impl ClassificationRules {
    pub fn classify(entry: &FileEntry, all_entries: &[FileEntry]) -> FileCategory {
        // System files take highest priority - NEVER suggest deleting
        if entry.is_system {
            return FileCategory::SystemFiles;
        }

        // Check for duplicate names
        if !all_entries.is_empty() {
            let same_name_count = all_entries.iter()
                .filter(|e| e.name == entry.name && e.path != entry.path && !e.is_dir)
                .count();
            if same_name_count > 0 && !entry.is_dir {
                return FileCategory::DuplicateName;
            }
        }
        
        Self::classify_by_pattern(entry)
    }

    fn classify_by_pattern(entry: &FileEntry) -> FileCategory {
        let path_str = entry.path.to_string_lossy().to_lowercase();
        let name_lower = entry.name.to_lowercase();

        // Check for Library folder contents
        if path_str.contains("/library/") {
            if path_str.contains("/library/caches") {
                return FileCategory::Cache;
            }
            if path_str.contains("/library/logs") {
                return FileCategory::LogFiles;
            }
            return FileCategory::LibraryFiles;
        }

        // Check for specific directories
        if path_str.contains("/downloads/") {
            return FileCategory::Downloads;
        }
        if path_str.contains("/documents/") {
            return FileCategory::Documents;
        }
        if path_str.contains("node_modules") {
            return FileCategory::NodeModules;
        }

        // Check for cache directories
        if path_str.contains("cache") || path_str.contains(".cache") {
            return FileCategory::Cache;
        }

        // Check for build artifacts
        if Self::is_build_artifact(&path_str, &name_lower) {
            return FileCategory::BuildArtifacts;
        }

        // Check for package caches
        if Self::is_package_cache(&path_str) {
            return FileCategory::PackageCache;
        }

        // Check for temp files
        if Self::is_temp_file(&path_str, &name_lower) {
            return FileCategory::TempFiles;
        }

        // Check for log files
        if Self::is_log_file(&path_str, &name_lower) {
            return FileCategory::LogFiles;
        }

        // Check for archives
        if Self::is_archive(&name_lower) {
            return FileCategory::Archives;
        }

        // Check for media files
        if Self::is_media_file(&name_lower) {
            return FileCategory::Media;
        }

        // Check for hidden files (config files)
        if entry.is_hidden {
            return FileCategory::HiddenFiles;
        }

        // Check for large files (>100MB)
        if entry.size > 100 * 1024 * 1024 {
            return FileCategory::LargeFiles;
        }

        // Check for old files (>1 year)
        let age_days = (chrono::Local::now() - entry.modified).num_days();
        if age_days > 365 {
            return FileCategory::OldFiles;
        }

        FileCategory::Regular
    }

    fn is_build_artifact(path_str: &str, name_lower: &str) -> bool {
        path_str.contains("target/debug") 
            || path_str.contains("target/release")
            || path_str.contains("/build/")
            || path_str.contains("/dist/")
            || path_str.contains(".next/")
            || path_str.contains("__pycache__")
            || path_str.contains(".pyc")
            || name_lower == "target" 
            || name_lower == "build"
            || name_lower == "dist"
    }

    fn is_package_cache(path_str: &str) -> bool {
        path_str.contains(".cargo/registry") 
            || path_str.contains(".npm") 
            || path_str.contains(".yarn/cache")
            || path_str.contains("pip/cache")
            || path_str.contains(".gradle/caches")
            || path_str.contains(".m2/repository")
            || path_str.contains("cocoapods/repos")
            || path_str.contains(".pub-cache")
    }

    fn is_temp_file(path_str: &str, name_lower: &str) -> bool {
        path_str.contains("/tmp/") 
            || path_str.contains("/temp/") 
            || name_lower.starts_with("tmp")
            || name_lower.starts_with("temp")
            || name_lower.ends_with(".tmp")
            || name_lower.ends_with(".temp")
            || name_lower.ends_with(".swp")
            || name_lower.ends_with("~")
    }

    fn is_log_file(path_str: &str, name_lower: &str) -> bool {
        name_lower.ends_with(".log") 
            || path_str.contains("/logs/")
            || name_lower.ends_with(".log.gz")
            || name_lower.ends_with(".log.1")
            || name_lower.ends_with(".log.old")
    }

    fn is_archive(name_lower: &str) -> bool {
        name_lower.ends_with(".zip")
            || name_lower.ends_with(".tar")
            || name_lower.ends_with(".tar.gz")
            || name_lower.ends_with(".tgz")
            || name_lower.ends_with(".rar")
            || name_lower.ends_with(".7z")
            || name_lower.ends_with(".dmg")
            || name_lower.ends_with(".iso")
    }

    fn is_media_file(name_lower: &str) -> bool {
        // Video
        name_lower.ends_with(".mp4")
            || name_lower.ends_with(".mov")
            || name_lower.ends_with(".avi")
            || name_lower.ends_with(".mkv")
            // Audio
            || name_lower.ends_with(".mp3")
            || name_lower.ends_with(".wav")
            || name_lower.ends_with(".flac")
            // Images
            || name_lower.ends_with(".jpg")
            || name_lower.ends_with(".jpeg")
            || name_lower.ends_with(".png")
            || name_lower.ends_with(".gif")
            || name_lower.ends_with(".heic")
            || name_lower.ends_with(".raw")
    }
}
