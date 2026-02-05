use std::collections::HashMap;

#[derive(PartialEq, Clone)]
#[allow(dead_code)]
pub enum AppState {
    Home,           // New home screen with options
    PathInput,      // Custom path input mode
    Scanning,
    ScanDetails,    // Detailed scan results with comprehensive stats
    Viewing,
    CategoryView,
    Deleting,
    Confirmation,
    SystemWarning,
    AllFiles,       // New: Dedicated all files view
    Search,         // New: Search mode
}

#[derive(PartialEq, Clone)]
pub enum ViewMode {
    AllFiles,
    Categories,
}

#[derive(PartialEq, Clone)]
pub enum ScanOption {
    FullDisk,
    HomeDirectory,
    CustomPath,
    QuickScan,
    LargeFiles,
    OldFiles,
}

impl Default for ScanOption {
    fn default() -> Self {
        ScanOption::HomeDirectory
    }
}

#[derive(Clone)]
pub struct HomeMenuState {
    pub options: Vec<ScanOption>,
    pub selected_option: usize,
    pub custom_path: String,
    pub path_suggestions: Vec<String>,
    pub min_size_mb: u64,
    pub max_depth: usize,
    pub include_hidden: bool,
    pub storage_info: StorageInfo,
}

impl Default for HomeMenuState {
    fn default() -> Self {
        Self {
            options: vec![
                ScanOption::FullDisk,
                ScanOption::HomeDirectory,
                ScanOption::CustomPath,
                ScanOption::QuickScan,
                ScanOption::LargeFiles,
                ScanOption::OldFiles,
            ],
            selected_option: 1,  // Default to Home Directory
            custom_path: String::new(),
            path_suggestions: Vec::new(),
            min_size_mb: 1,
            max_depth: 0,
            include_hidden: true,
            storage_info: StorageInfo::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct ScanProgressSnapshot {
    pub current_path: String,
    pub files_scanned: usize,
    pub dirs_scanned: usize,
    pub total_size_scanned: u64,
    pub entries_count: usize,
    pub top_entries: Vec<(String, u64, String)>,  // (name, size, category)
    pub category_sizes: HashMap<String, u64>,  // category name -> total size
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
