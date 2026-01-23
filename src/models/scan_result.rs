use super::FileEntry;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub entries: Vec<FileEntry>,
    pub total_size: u64,
    pub total_files: usize,
    pub total_dirs: usize,
    pub hidden_count: usize,
    pub system_count: usize,
}
