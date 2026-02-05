// App State - Core application state struct and constructors

use crate::models::{FileEntry, ScanProgress, ScanResult};
use crate::analyzer::FileCategory;
use super::super::types::*;
use super::super::screens::AllFilesState;
use ratatui::{layout::Rect, widgets::ListState};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Sort mode for browse view
#[derive(Clone, Copy, PartialEq)]
pub enum BrowseSortMode {
    SizeDesc,
    SizeAsc,
    NameAsc,
    NameDesc,
    DateDesc,
    DateAsc,
}

impl Default for BrowseSortMode {
    fn default() -> Self {
        BrowseSortMode::SizeDesc
    }
}

impl BrowseSortMode {
    pub fn name(&self) -> &'static str {
        match self {
            BrowseSortMode::SizeDesc => "Size ↓",
            BrowseSortMode::SizeAsc => "Size ↑",
            BrowseSortMode::NameAsc => "Name A-Z",
            BrowseSortMode::NameDesc => "Name Z-A",
            BrowseSortMode::DateDesc => "Date ↓",
            BrowseSortMode::DateAsc => "Date ↑",
        }
    }
    
    pub fn cycle(&self) -> Self {
        match self {
            BrowseSortMode::SizeDesc => BrowseSortMode::SizeAsc,
            BrowseSortMode::SizeAsc => BrowseSortMode::NameAsc,
            BrowseSortMode::NameAsc => BrowseSortMode::NameDesc,
            BrowseSortMode::NameDesc => BrowseSortMode::DateDesc,
            BrowseSortMode::DateDesc => BrowseSortMode::DateAsc,
            BrowseSortMode::DateAsc => BrowseSortMode::SizeDesc,
        }
    }
}

pub struct App {
    pub state: AppState,
    pub scan_result: Option<ScanResult>,
    pub scan_path: PathBuf,
    pub current_path: PathBuf,
    pub navigation_stack: Vec<PathBuf>,
    pub list_state: ListState,
    pub category_state: ListState,
    pub selected_category: Option<FileCategory>,
    pub categories: HashMap<FileCategory, Vec<FileEntry>>,
    pub marked_for_deletion: Vec<usize>,
    pub recommendations: Vec<String>,
    pub status_message: String,
    pub show_help: bool,
    pub current_view: ViewMode,
    pub scan_progress: Arc<Mutex<ScanProgress>>,
    pub system_warning_message: String,
    pub pending_system_deletions: Vec<usize>,
    pub show_hidden: bool,
    pub last_progress_snapshot: ScanProgressSnapshot,
    pub storage_info: StorageInfo,
    pub home_menu: HomeMenuState,
    pub path_input: String,
    pub path_cursor: usize,
    pub scan_scroll_offset: usize,
    pub frame_count: u32,
    pub all_files_state: AllFilesState,
    #[allow(dead_code)]
    pub search_active: bool,
    #[allow(dead_code)]
    pub search_query: String,
    #[allow(dead_code)]
    pub last_list_area: Option<Rect>,
    pub browse_sort_mode: BrowseSortMode,
    pub browse_search_active: bool,
    pub browse_search_query: String,
}

impl App {
    pub fn new(scan_path: PathBuf) -> Self {
        let storage_info = StorageInfo::from_path(&scan_path);
        let mut home_menu = HomeMenuState::default();
        home_menu.storage_info = storage_info.clone();
        
        Self {
            state: AppState::Home,
            scan_result: None,
            current_path: scan_path.clone(),
            navigation_stack: Vec::new(),
            scan_path,
            list_state: ListState::default(),
            category_state: ListState::default(),
            selected_category: None,
            categories: HashMap::new(),
            marked_for_deletion: Vec::new(),
            recommendations: Vec::new(),
            status_message: String::new(),
            show_help: false,
            current_view: ViewMode::AllFiles,
            scan_progress: Arc::new(Mutex::new(ScanProgress::default())),
            system_warning_message: String::new(),
            pending_system_deletions: Vec::new(),
            show_hidden: true,
            last_progress_snapshot: ScanProgressSnapshot::default(),
            storage_info,
            home_menu,
            path_input: String::new(),
            path_cursor: 0,
            scan_scroll_offset: 0,
            frame_count: 0,
            all_files_state: AllFilesState::default(),
            search_active: false,
            search_query: String::new(),
            last_list_area: None,
            browse_sort_mode: BrowseSortMode::default(),
            browse_search_active: false,
            browse_search_query: String::new(),
        }
    }
    
    pub fn switch_view(&mut self) {
        self.current_view = match self.current_view {
            ViewMode::AllFiles => ViewMode::Categories,
            ViewMode::Categories => ViewMode::AllFiles,
        };
        self.status_message = match self.current_view {
            ViewMode::AllFiles => "File Browser".to_string(),
            ViewMode::Categories => "Categories · Enter to drill down".to_string(),
        };
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.status_message = if self.show_hidden {
            "Showing hidden files".to_string()
        } else {
            "Hidden files filtered".to_string()
        };
    }
    
    pub fn get_scan_path_from_option(&self) -> PathBuf {
        match &self.home_menu.options[self.home_menu.selected_option] {
            ScanOption::FullDisk => PathBuf::from("/"),
            ScanOption::HomeDirectory => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            ScanOption::CustomPath => {
                if self.home_menu.custom_path.is_empty() {
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
                } else {
                    PathBuf::from(&self.home_menu.custom_path)
                }
            },
            ScanOption::QuickScan => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            ScanOption::LargeFiles => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            ScanOption::OldFiles => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }
    
    pub fn update_path_suggestions(&mut self) {
        if self.path_input.is_empty() {
            self.home_menu.path_suggestions = vec![
                "/".to_string(),
                dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            ];
            return;
        }
        
        let path = PathBuf::from(&self.path_input);
        let parent = if path.is_dir() {
            path.clone()
        } else {
            path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("/"))
        };
        
        if let Ok(entries) = std::fs::read_dir(&parent) {
            self.home_menu.path_suggestions = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path().to_string_lossy().to_string())
                .filter(|p| p.starts_with(&self.path_input))
                .take(5)
                .collect();
        }
    }
}
