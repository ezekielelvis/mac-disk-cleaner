use chrono::{DateTime, Local};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_system: bool,
    pub modified: DateTime<Local>,
    pub name: String,
}
