//! Plain serializable request/response structs exchanged with the browser.

use crate::models::StorageInfo;
use serde::{Deserialize, Serialize};

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
    /// Live size-by-group breakdown of what has been discovered so far,
    /// sorted largest first. Drives the scanning chart.
    pub categories: Vec<ProgressCategoryDto>,
}

/// One group in the live scan breakdown (e.g. "Cache", "Media", "node_modules").
#[derive(Serialize)]
pub struct ProgressCategoryDto {
    pub name: String,
    pub size: u64,
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
