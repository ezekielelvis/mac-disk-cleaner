use super::FileEntry;

#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub current_path: String,
    pub files_scanned: usize,
    pub dirs_scanned: usize,
    pub total_size_scanned: u64,
    pub is_complete: bool,
    pub entries: Vec<FileEntry>,
}

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            current_path: String::new(),
            files_scanned: 0,
            dirs_scanned: 0,
            total_size_scanned: 0,
            is_complete: false,
            entries: Vec::new(),
        }
    }
}
