use crate::scanner::FileEntry;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Cache,
    TempFiles,
    LargeFiles,
    OldFiles,
    DuplicateName,
    LogFiles,
    BuildArtifacts,
    NodeModules,
    PackageCache,
    HiddenFiles,
    SystemFiles,
    LibraryFiles,
    Downloads,
    Documents,
    Media,
    Archives,
    Regular,
}

impl FileCategory {
    pub fn as_str(&self) -> &str {
        match self {
            FileCategory::Cache => "🗑️ Cache",
            FileCategory::TempFiles => "🌡️ Temp Files",
            FileCategory::LargeFiles => "📦 Large Files",
            FileCategory::OldFiles => "📅 Old Files",
            FileCategory::DuplicateName => "👯 Duplicate Names",
            FileCategory::LogFiles => "📜 Log Files",
            FileCategory::BuildArtifacts => "🔨 Build Artifacts",
            FileCategory::NodeModules => "📦 node_modules",
            FileCategory::PackageCache => "📥 Package Cache",
            FileCategory::HiddenFiles => "👁️ Hidden Files",
            FileCategory::SystemFiles => "⚙️ System Files",
            FileCategory::LibraryFiles => "📚 Library Files",
            FileCategory::Downloads => "⬇️ Downloads",
            FileCategory::Documents => "📄 Documents",
            FileCategory::Media => "🎬 Media",
            FileCategory::Archives => "🗜️ Archives",
            FileCategory::Regular => "📁 Regular",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            FileCategory::Cache => Color::Yellow,
            FileCategory::TempFiles => Color::Red,
            FileCategory::LargeFiles => Color::Magenta,
            FileCategory::OldFiles => Color::Cyan,
            FileCategory::DuplicateName => Color::Blue,
            FileCategory::LogFiles => Color::LightYellow,
            FileCategory::BuildArtifacts => Color::LightRed,
            FileCategory::NodeModules => Color::LightMagenta,
            FileCategory::PackageCache => Color::LightCyan,
            FileCategory::HiddenFiles => Color::Gray,
            FileCategory::SystemFiles => Color::Red,
            FileCategory::LibraryFiles => Color::LightBlue,
            FileCategory::Downloads => Color::Green,
            FileCategory::Documents => Color::White,
            FileCategory::Media => Color::LightGreen,
            FileCategory::Archives => Color::Yellow,
            FileCategory::Regular => Color::White,
        }
    }

    pub fn is_safe_to_delete(&self) -> bool {
        match self {
            FileCategory::Cache => true,
            FileCategory::TempFiles => true,
            FileCategory::BuildArtifacts => true,
            FileCategory::NodeModules => true,
            FileCategory::PackageCache => true,
            FileCategory::LogFiles => true,
            FileCategory::OldFiles => true,
            FileCategory::Archives => true,
            FileCategory::Downloads => true,
            FileCategory::DuplicateName => true,
            FileCategory::LargeFiles => false, // Need review
            FileCategory::HiddenFiles => false, // Might be config
            FileCategory::SystemFiles => false, // Dangerous
            FileCategory::LibraryFiles => false, // App data
            FileCategory::Documents => false, // User data
            FileCategory::Media => false, // User data
            FileCategory::Regular => false,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            FileCategory::Cache => "Temporary cache files that can be safely deleted. Will be regenerated.",
            FileCategory::TempFiles => "Temporary files. Usually safe to delete.",
            FileCategory::LargeFiles => "Large files over 100MB. Review before deleting.",
            FileCategory::OldFiles => "Files not accessed in over a year. May be obsolete.",
            FileCategory::DuplicateName => "Files with same name in different locations. May be duplicates.",
            FileCategory::LogFiles => "Application log files. Can grow large over time.",
            FileCategory::BuildArtifacts => "Compiled code and build outputs. Can be regenerated.",
            FileCategory::NodeModules => "JavaScript dependencies. Can be reinstalled with npm/yarn.",
            FileCategory::PackageCache => "Downloaded packages. Can be re-downloaded when needed.",
            FileCategory::HiddenFiles => "Hidden files (start with .). May contain important configs.",
            FileCategory::SystemFiles => "⚠️ SYSTEM FILES - Required for OS operation. DO NOT DELETE!",
            FileCategory::LibraryFiles => "Application data in Library folder. Deleting may break apps.",
            FileCategory::Downloads => "Downloaded files. Review before deleting.",
            FileCategory::Documents => "User documents. Be careful!",
            FileCategory::Media => "Photos, videos, audio files. User content.",
            FileCategory::Archives => "Compressed archives (.zip, .tar, etc). May be backup copies.",
            FileCategory::Regular => "Regular files.",
        }
    }
}

pub struct Analyzer;

impl Analyzer {
    pub fn categorize_file(entry: &FileEntry) -> FileCategory {
        Self::categorize_file_with_context(entry, &[])
    }

    pub fn categorize_file_with_context(entry: &FileEntry, all_entries: &[FileEntry]) -> FileCategory {
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
        
        Self::categorize_file_internal(entry)
    }

    fn categorize_file_internal(entry: &FileEntry) -> FileCategory {
        let path_str = entry.path.to_string_lossy().to_lowercase();
        let name_lower = entry.name.to_lowercase();

        // Check for Library folder contents
        if path_str.contains("/library/") {
            // Specific library subfolders
            if path_str.contains("/library/caches") {
                return FileCategory::Cache;
            }
            if path_str.contains("/library/logs") {
                return FileCategory::LogFiles;
            }
            return FileCategory::LibraryFiles;
        }

        // Check for Downloads folder
        if path_str.contains("/downloads/") {
            return FileCategory::Downloads;
        }

        // Check for Documents folder
        if path_str.contains("/documents/") {
            return FileCategory::Documents;
        }

        // Check for node_modules
        if path_str.contains("node_modules") {
            return FileCategory::NodeModules;
        }

        // Check for cache directories
        if path_str.contains("cache") || path_str.contains(".cache") {
            return FileCategory::Cache;
        }

        // Check for build artifacts
        if path_str.contains("target/debug") 
            || path_str.contains("target/release")
            || path_str.contains("/build/")
            || path_str.contains("/dist/")
            || path_str.contains(".next/")
            || path_str.contains("__pycache__")
            || path_str.contains(".pyc")
            || name_lower == "target" 
            || name_lower == "build"
            || name_lower == "dist" {
            return FileCategory::BuildArtifacts;
        }

        // Check for package caches
        if path_str.contains(".cargo/registry") 
            || path_str.contains(".npm") 
            || path_str.contains(".yarn/cache")
            || path_str.contains("pip/cache")
            || path_str.contains(".gradle/caches")
            || path_str.contains(".m2/repository")
            || path_str.contains("cocoapods/repos")
            || path_str.contains(".pub-cache") {
            return FileCategory::PackageCache;
        }

        // Check for temp files
        if path_str.contains("/tmp/") 
            || path_str.contains("/temp/") 
            || name_lower.starts_with("tmp")
            || name_lower.starts_with("temp")
            || name_lower.ends_with(".tmp")
            || name_lower.ends_with(".temp")
            || name_lower.ends_with(".swp")
            || name_lower.ends_with("~") {
            return FileCategory::TempFiles;
        }

        // Check for log files
        if name_lower.ends_with(".log") 
            || path_str.contains("/logs/")
            || name_lower.ends_with(".log.gz")
            || name_lower.ends_with(".log.1")
            || name_lower.ends_with(".log.old") {
            return FileCategory::LogFiles;
        }

        // Check for archives
        if name_lower.ends_with(".zip")
            || name_lower.ends_with(".tar")
            || name_lower.ends_with(".tar.gz")
            || name_lower.ends_with(".tgz")
            || name_lower.ends_with(".rar")
            || name_lower.ends_with(".7z")
            || name_lower.ends_with(".dmg")
            || name_lower.ends_with(".iso") {
            return FileCategory::Archives;
        }

        // Check for media files
        if name_lower.ends_with(".mp4")
            || name_lower.ends_with(".mov")
            || name_lower.ends_with(".avi")
            || name_lower.ends_with(".mkv")
            || name_lower.ends_with(".mp3")
            || name_lower.ends_with(".wav")
            || name_lower.ends_with(".flac")
            || name_lower.ends_with(".jpg")
            || name_lower.ends_with(".jpeg")
            || name_lower.ends_with(".png")
            || name_lower.ends_with(".gif")
            || name_lower.ends_with(".heic")
            || name_lower.ends_with(".raw") {
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

    pub fn group_by_category(entries: &[FileEntry]) -> HashMap<FileCategory, Vec<FileEntry>> {
        let mut groups: HashMap<FileCategory, Vec<FileEntry>> = HashMap::new();

        for entry in entries {
            let category = Self::categorize_file_with_context(entry, entries);
            groups.entry(category).or_default().push(entry.clone());
        }

        groups
    }

    pub fn find_large_directories(entries: &[FileEntry]) -> Vec<(String, u64)> {
        let mut dir_sizes: HashMap<String, u64> = HashMap::new();

        for entry in entries {
            if !entry.is_dir {
                if let Some(parent) = entry.path.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    *dir_sizes.entry(parent_str).or_default() += entry.size;
                }
            }
        }

        let mut dirs: Vec<(String, u64)> = dir_sizes.into_iter().collect();
        dirs.sort_by(|a, b| b.1.cmp(&a.1));
        dirs.truncate(20);
        dirs
    }

    pub fn get_recommendations(entries: &[FileEntry]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let groups = Self::group_by_category(entries);
        
        // Add large directories recommendation
        let large_dirs = Self::find_large_directories(entries);
        if !large_dirs.is_empty() {
            let top_dir = &large_dirs[0];
            recommendations.push(format!(
                "📂 Largest directory: {} ({})",
                top_dir.0,
                humansize::format_size(top_dir.1, humansize::DECIMAL)
            ));
        }

        // Count system files
        if let Some(system) = groups.get(&FileCategory::SystemFiles) {
            recommendations.push(format!(
                "🛑 {} system files found - DO NOT DELETE",
                system.len()
            ));
        }

        // Count hidden files
        if let Some(hidden) = groups.get(&FileCategory::HiddenFiles) {
            let total_size: u64 = hidden.iter().map(|e| e.size).sum();
            recommendations.push(format!(
                "👁️ {} hidden files using {} - review carefully",
                hidden.len(),
                humansize::format_size(total_size, humansize::DECIMAL)
            ));
        }

        if let Some(node_modules) = groups.get(&FileCategory::NodeModules) {
            let total_size: u64 = node_modules.iter().map(|e| e.size).sum();
            recommendations.push(format!(
                "📦 {} node_modules entries using {} - safe to delete",
                node_modules.len(),
                humansize::format_size(total_size, humansize::DECIMAL)
            ));
        }

        if let Some(cache) = groups.get(&FileCategory::Cache) {
            let total_size: u64 = cache.iter().map(|e| e.size).sum();
            recommendations.push(format!(
                "🗑️ Cache files using {} - safe to delete",
                humansize::format_size(total_size, humansize::DECIMAL)
            ));
        }

        if let Some(builds) = groups.get(&FileCategory::BuildArtifacts) {
            let total_size: u64 = builds.iter().map(|e| e.size).sum();
            recommendations.push(format!(
                "🔨 Build artifacts using {} - can be regenerated",
                humansize::format_size(total_size, humansize::DECIMAL)
            ));
        }

        if let Some(logs) = groups.get(&FileCategory::LogFiles) {
            let total_size: u64 = logs.iter().map(|e| e.size).sum();
            if total_size > 50 * 1024 * 1024 {
                recommendations.push(format!(
                    "📜 Log files using {} - consider cleaning",
                    humansize::format_size(total_size, humansize::DECIMAL)
                ));
            }
        }

        if let Some(duplicates) = groups.get(&FileCategory::DuplicateName) {
            recommendations.push(format!(
                "👯 {} files with duplicate names - review for duplicates",
                duplicates.len()
            ));
        }

        if let Some(downloads) = groups.get(&FileCategory::Downloads) {
            let total_size: u64 = downloads.iter().map(|e| e.size).sum();
            if total_size > 500 * 1024 * 1024 {
                recommendations.push(format!(
                    "⬇️ Downloads using {} - review and clean",
                    humansize::format_size(total_size, humansize::DECIMAL)
                ));
            }
        }

        if let Some(archives) = groups.get(&FileCategory::Archives) {
            let total_size: u64 = archives.iter().map(|e| e.size).sum();
            if total_size > 200 * 1024 * 1024 {
                recommendations.push(format!(
                    "🗜️ Archives using {} - may be extractable or deletable",
                    humansize::format_size(total_size, humansize::DECIMAL)
                ));
            }
        }

        recommendations
    }

    /// Calculate potential space savings from safe-to-delete categories
    pub fn calculate_safe_savings(entries: &[FileEntry]) -> u64 {
        let groups = Self::group_by_category(entries);
        let mut total = 0u64;

        for (category, files) in groups {
            if category.is_safe_to_delete() {
                total += files.iter().map(|f| f.size).sum::<u64>();
            }
        }

        total
    }
}
