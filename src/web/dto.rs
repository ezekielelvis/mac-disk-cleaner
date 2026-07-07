use crate::analyzer::FileCategory;
use crate::models::{FileEntry, StorageInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Every category variant, used to build the breakdown and resolve colors.
pub const ALL_CATEGORIES: [FileCategory; 17] = [
    FileCategory::Cache,
    FileCategory::TempFiles,
    FileCategory::LargeFiles,
    FileCategory::OldFiles,
    FileCategory::DuplicateName,
    FileCategory::LogFiles,
    FileCategory::BuildArtifacts,
    FileCategory::NodeModules,
    FileCategory::PackageCache,
    FileCategory::HiddenFiles,
    FileCategory::SystemFiles,
    FileCategory::LibraryFiles,
    FileCategory::Downloads,
    FileCategory::Documents,
    FileCategory::Media,
    FileCategory::Archives,
    FileCategory::Regular,
];

/// Maximum number of individual files returned to the browser.
const MAX_FILES: usize = 500;

#[derive(Serialize)]
pub struct StorageDto {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub percent: f64,
}

impl From<&StorageInfo> for StorageDto {
    fn from(s: &StorageInfo) -> Self {
        Self {
            total: s.total_space,
            used: s.used_space,
            available: s.available_space,
            percent: s.usage_percent() * 100.0,
        }
    }
}

#[derive(Serialize)]
pub struct ProgressDto {
    pub files: usize,
    pub dirs: usize,
    pub size: u64,
    pub current_path: String,
    pub complete: bool,
}

#[derive(Serialize)]
pub struct CategoryDto {
    pub name: String,
    pub color: String,
    pub description: String,
    pub size: u64,
    pub count: usize,
    pub safe: bool,
}

#[derive(Serialize)]
pub struct DirDto {
    pub name: String,
    pub path: String,
    pub size: u64,
}

#[derive(Serialize)]
pub struct FileDto {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub category: String,
    pub color: String,
    pub safe: bool,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_system: bool,
    pub modified: String,
}

#[derive(Serialize)]
pub struct ResultsDto {
    pub scan_path: String,
    pub total_size: u64,
    pub total_files: usize,
    pub total_dirs: usize,
    pub safe_savings: u64,
    pub storage: StorageDto,
    pub categories: Vec<CategoryDto>,
    pub directories: Vec<DirDto>,
    pub files: Vec<FileDto>,
    pub recommendations: Vec<String>,
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub path: String,
    #[serde(default = "default_min_size")]
    pub min_size_mb: u64,
    #[serde(default)]
    pub max_depth: usize,
}

fn default_min_size() -> u64 {
    1
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub paths: Vec<String>,
}

#[derive(Serialize)]
pub struct DeleteResult {
    pub path: String,
    pub success: bool,
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub results: Vec<DeleteResult>,
    pub freed: u64,
    pub deleted: usize,
}

fn human(bytes: u64) -> String {
    humansize::format_size(bytes, humansize::DECIMAL)
}

/// Build the full results payload from categorized entries and the scan path.
///
/// `entries` are paired with their category (computed once, without duplicate
/// context) so the breakdown, directory rollup and file list stay O(n).
pub fn build_results(scan_path: &Path, entries: &[(FileEntry, FileCategory)]) -> ResultsDto {
    let storage = StorageInfo::from_path(scan_path);

    let total_files = entries.iter().filter(|(e, _)| !e.is_dir).count();
    let total_dirs = entries.iter().filter(|(e, _)| e.is_dir).count();
    let total_size: u64 = entries.iter().filter(|(e, _)| !e.is_dir).map(|(e, _)| e.size).sum();

    // Category breakdown
    let mut cat_size: HashMap<FileCategory, u64> = HashMap::new();
    let mut cat_count: HashMap<FileCategory, usize> = HashMap::new();
    for (entry, cat) in entries {
        *cat_size.entry(*cat).or_default() += entry.size;
        *cat_count.entry(*cat).or_default() += 1;
    }

    let mut categories: Vec<CategoryDto> = ALL_CATEGORIES
        .iter()
        .filter_map(|cat| {
            let size = *cat_size.get(cat).unwrap_or(&0);
            let count = *cat_count.get(cat).unwrap_or(&0);
            if count == 0 {
                return None;
            }
            Some(CategoryDto {
                name: cat.as_str().to_string(),
                color: cat.color().to_string(),
                description: cat.description().to_string(),
                size,
                count,
                safe: cat.is_safe_to_delete(),
            })
        })
        .collect();
    categories.sort_by(|a, b| b.size.cmp(&a.size));

    let safe_savings: u64 = entries
        .iter()
        .filter(|(e, c)| !e.is_dir && c.is_safe_to_delete())
        .map(|(e, _)| e.size)
        .sum();

    // Top-level directory rollup relative to the scan root.
    let mut dir_sizes: HashMap<String, u64> = HashMap::new();
    for (entry, _) in entries {
        if entry.is_dir {
            continue;
        }
        if let Ok(rel) = entry.path.strip_prefix(scan_path) {
            if let Some(first) = rel.components().next() {
                let name = first.as_os_str().to_string_lossy().to_string();
                *dir_sizes.entry(name).or_default() += entry.size;
            }
        }
    }
    let mut directories: Vec<DirDto> = dir_sizes
        .into_iter()
        .map(|(name, size)| {
            let path = scan_path.join(&name).to_string_lossy().to_string();
            DirDto { name, path, size }
        })
        .collect();
    directories.sort_by(|a, b| b.size.cmp(&a.size));
    directories.truncate(50);

    // Largest individual files.
    let mut sorted: Vec<&(FileEntry, FileCategory)> =
        entries.iter().filter(|(e, _)| !e.is_dir).collect();
    sorted.sort_by(|a, b| b.0.size.cmp(&a.0.size));
    let files: Vec<FileDto> = sorted
        .into_iter()
        .take(MAX_FILES)
        .map(|(e, cat)| FileDto {
            path: e.path.to_string_lossy().to_string(),
            name: e.name.clone(),
            size: e.size,
            category: cat.as_str().to_string(),
            color: cat.color().to_string(),
            safe: cat.is_safe_to_delete(),
            is_dir: e.is_dir,
            is_hidden: e.is_hidden,
            is_system: e.is_system,
            modified: e.modified.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect();

    let recommendations = build_recommendations(&categories, &directories);

    ResultsDto {
        scan_path: scan_path.to_string_lossy().to_string(),
        total_size,
        total_files,
        total_dirs,
        safe_savings,
        storage: StorageDto::from(&storage),
        categories,
        directories,
        files,
        recommendations,
    }
}

fn build_recommendations(categories: &[CategoryDto], directories: &[DirDto]) -> Vec<String> {
    let mut recs = Vec::new();

    if let Some(top) = directories.first() {
        if top.size > 0 {
            recs.push(format!("📂 Largest directory: {} ({})", top.name, human(top.size)));
        }
    }

    let find = |needle: &str| categories.iter().find(|c| c.name.contains(needle));

    if let Some(c) = find("System Files") {
        recs.push(format!("🛑 {} system files found — DO NOT DELETE", c.count));
    }
    if let Some(c) = find("node_modules") {
        recs.push(format!(
            "📦 {} node_modules entries using {} — safe to delete",
            c.count,
            human(c.size)
        ));
    }
    if let Some(c) = find("Build Artifacts") {
        recs.push(format!("🔨 Build artifacts using {} — can be regenerated", human(c.size)));
    }
    if let Some(c) = find("Cache") {
        recs.push(format!("🗑️ Cache files using {} — safe to delete", human(c.size)));
    }
    if let Some(c) = find("Package Cache") {
        recs.push(format!("📥 Package cache using {} — can be re-downloaded", human(c.size)));
    }
    if let Some(c) = find("Log Files") {
        if c.size > 50 * 1024 * 1024 {
            recs.push(format!("📜 Log files using {} — consider cleaning", human(c.size)));
        }
    }
    if let Some(c) = find("Hidden Files") {
        recs.push(format!(
            "👁️ {} hidden files using {} — review carefully",
            c.count,
            human(c.size)
        ));
    }

    if recs.is_empty() {
        recs.push("✨ Nothing obvious to clean up here.".to_string());
    }
    recs
}
