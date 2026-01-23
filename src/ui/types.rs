#[derive(PartialEq, Clone)]
pub enum AppState {
    Scanning,
    Viewing,
    CategoryView,
    Deleting,
    Confirmation,
    SystemWarning,
}

#[derive(PartialEq, Clone)]
pub enum ViewMode {
    AllFiles,
    Categories,
}

#[derive(Clone, Default)]
pub struct ScanProgressSnapshot {
    pub current_path: String,
    pub files_scanned: usize,
    pub dirs_scanned: usize,
    pub total_size_scanned: u64,
    pub entries_count: usize,
    pub top_entries: Vec<(String, u64, String)>,  // (name, size, category)
}

#[derive(Clone, Default)]
pub struct StorageInfo {
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
}

impl StorageInfo {
    pub fn from_path(path: &std::path::Path) -> Self {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem::MaybeUninit;
            
            if let Ok(path_str) = CString::new(path.to_string_lossy().as_bytes()) {
                let mut stat: MaybeUninit<libc::statfs> = MaybeUninit::uninit();
                unsafe {
                    if libc::statfs(path_str.as_ptr(), stat.as_mut_ptr()) == 0 {
                        let stat = stat.assume_init();
                        let block_size = stat.f_bsize as u64;
                        let total = stat.f_blocks * block_size;
                        let available = stat.f_bavail * block_size;
                        return Self {
                            total_space: total,
                            available_space: available,
                            used_space: total.saturating_sub(available),
                        };
                    }
                }
            }
        }
        Self::default()
    }
    
    pub fn usage_percent(&self) -> f64 {
        if self.total_space == 0 {
            0.0
        } else {
            self.used_space as f64 / self.total_space as f64
        }
    }
}
