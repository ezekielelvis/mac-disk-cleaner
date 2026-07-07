use crate::models::FileEntry;
use super::category::FileCategory;
use super::rules::ClassificationRules;
use std::collections::HashMap;

pub struct Analyzer;

impl Analyzer {
    pub fn categorize_file(entry: &FileEntry) -> FileCategory {
        ClassificationRules::classify(entry, &[])
    }

    pub fn categorize_file_with_context(entry: &FileEntry, all_entries: &[FileEntry]) -> FileCategory {
        ClassificationRules::classify(entry, all_entries)
    }

    /// Categorize an owned batch of entries in O(n), preserving duplicate-name
    /// detection by computing the name-frequency map a single time.
    pub fn categorize_all(entries: Vec<FileEntry>) -> Vec<(FileEntry, FileCategory)> {
        let name_counts = ClassificationRules::build_name_counts(&entries);
        entries
            .into_iter()
            .map(|entry| {
                let category = ClassificationRules::classify_with_name_counts(&entry, &name_counts);
                (entry, category)
            })
            .collect()
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

        Self::add_category_recommendations(&mut recommendations, &groups);
        recommendations
    }

    fn add_category_recommendations(
        recommendations: &mut Vec<String>,
        groups: &HashMap<FileCategory, Vec<FileEntry>>,
    ) {
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
    }
}
