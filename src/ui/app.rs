use crate::analyzer::{Analyzer, FileCategory};
use crate::cleaner::Cleaner;
use crate::models::{FileEntry, ScanProgress, ScanResult};
use crate::scanner::{Scanner, get_system_warning};
use super::types::*;
use super::render_home::{render_home, render_path_input};
use super::render_scanning::render_scanning_enhanced;
use super::render_results::{render_results_view, render_scan_complete, render_help_overlay, render_confirmation_dialog, render_system_warning_dialog};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{ListState, Clear},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct App {
    pub state: AppState,
    pub scan_result: Option<ScanResult>,
    pub scan_path: PathBuf,
    pub current_path: PathBuf,           // Current browsing path
    pub navigation_stack: Vec<PathBuf>,  // For back navigation
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
}

impl App {
    fn new(scan_path: PathBuf) -> Self {
        let storage_info = StorageInfo::from_path(&scan_path);
        let mut home_menu = HomeMenuState::default();
        home_menu.storage_info = storage_info.clone();
        
        Self {
            state: AppState::Home,  // Start at home screen
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
        }
    }

    pub fn get_current_entries(&self) -> Vec<(usize, &FileEntry)> {
        if let Some(ref result) = self.scan_result {
            result.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    // Filter by current path (direct children only)
                    if let Some(parent) = e.path.parent() {
                        parent == self.current_path
                    } else {
                        false
                    }
                })
                .filter(|(_, e)| self.show_hidden || !e.is_hidden)
                .collect()
        } else {
            Vec::new()
        }
    }

    #[allow(dead_code)]
    fn get_visible_entries(&self) -> Vec<(usize, &FileEntry)> {
        if let Some(ref result) = self.scan_result {
            result.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| self.show_hidden || !e.is_hidden)
                .collect()
        } else {
            Vec::new()
        }
    }

    fn next_item(&mut self) {
        let items_len = match self.current_view {
            ViewMode::AllFiles => self.get_current_entries().len(),
            ViewMode::Categories => self.categories.len(),
        };

        if items_len == 0 {
            return;
        }

        let state = match self.current_view {
            ViewMode::AllFiles => &mut self.list_state,
            ViewMode::Categories => &mut self.category_state,
        };

        let i = match state.selected() {
            Some(i) => {
                if i >= items_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    fn previous_item(&mut self) {
        let items_len = match self.current_view {
            ViewMode::AllFiles => self.get_current_entries().len(),
            ViewMode::Categories => self.categories.len(),
        };

        if items_len == 0 {
            return;
        }

        let state = match self.current_view {
            ViewMode::AllFiles => &mut self.list_state,
            ViewMode::Categories => &mut self.category_state,
        };

        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    items_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    fn enter_folder(&mut self) {
        if self.current_view != ViewMode::AllFiles {
            return;
        }

        // Get the target path first to avoid borrow issues
        let target_path: Option<PathBuf> = {
            let current_entries = self.get_current_entries();
            if let Some(selected_idx) = self.list_state.selected() {
                if let Some((_, entry)) = current_entries.get(selected_idx) {
                    if entry.is_dir {
                        Some(entry.path.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(new_path) = target_path {
            self.navigation_stack.push(self.current_path.clone());
            self.current_path = new_path;
            self.list_state.select(Some(0));
            self.status_message = format!("📂 {}", self.current_path.to_string_lossy());
        }
    }

    fn go_back(&mut self) {
        if let Some(prev_path) = self.navigation_stack.pop() {
            self.current_path = prev_path;
            self.list_state.select(Some(0));
            self.status_message = format!("📂 {}", self.current_path.to_string_lossy());
        } else if self.current_path != self.scan_path {
            // Go to parent if possible
            if let Some(parent) = self.current_path.parent() {
                if parent.starts_with(&self.scan_path) || parent == self.scan_path {
                    self.current_path = parent.to_path_buf();
                    self.list_state.select(Some(0));
                    self.status_message = format!("📂 {}", self.current_path.to_string_lossy());
                }
            }
        }
    }

    fn toggle_mark(&mut self) {
        if self.current_view != ViewMode::AllFiles {
            return;
        }

        let current_entries = self.get_current_entries();
        if let Some(visible_idx) = self.list_state.selected() {
            if let Some((actual_idx, entry)) = current_entries.get(visible_idx) {
                if entry.is_system {
                    self.status_message = "⚠️  Cannot mark system file".to_string();
                    return;
                }

                if let Some(pos) = self.marked_for_deletion.iter().position(|&x| x == *actual_idx) {
                    self.marked_for_deletion.remove(pos);
                    self.status_message = format!("Unmarked · {} selected", self.marked_for_deletion.len());
                } else {
                    self.marked_for_deletion.push(*actual_idx);
                    self.status_message = format!("Marked · {} selected", self.marked_for_deletion.len());
                }
            }
        }
    }

    fn delete_marked(&mut self) {
        if self.marked_for_deletion.is_empty() {
            self.status_message = "No items selected".to_string();
            return;
        }

        if let Some(ref result) = self.scan_result {
            let system_files: Vec<usize> = self.marked_for_deletion.iter()
                .filter(|&&i| result.entries.get(i).map(|e| e.is_system).unwrap_or(false))
                .cloned()
                .collect();

            if !system_files.is_empty() {
                let system_entry = result.entries.get(system_files[0]).unwrap();
                self.system_warning_message = format!(
                    "🛑 SYSTEM FILE WARNING\n\n{} system file(s) selected\n\n{}\n\n{}\n\nPress Y to proceed (dangerous) or N to cancel",
                    system_files.len(),
                    system_entry.path.to_string_lossy(),
                    get_system_warning(&system_entry.path)
                        .unwrap_or_else(|| "Critical system file".to_string())
                );
                self.pending_system_deletions = system_files;
                self.state = AppState::SystemWarning;
                return;
            }

            let paths: Vec<_> = self.marked_for_deletion.iter()
                .filter_map(|&i| result.entries.get(i))
                .map(|e| e.path.as_path())
                .collect();

            let space_to_free = Cleaner::estimate_space_freed(&paths);
            self.status_message = format!(
                "Delete {} items? Free {} · Press Y to confirm, N to cancel",
                paths.len(),
                humansize::format_size(space_to_free, humansize::DECIMAL)
            );
            self.state = AppState::Confirmation;
        }
    }

    fn enter_category_view(&mut self) {
        if let Some(i) = self.category_state.selected() {
            let mut categories: Vec<_> = self.categories.keys().collect();
            categories.sort_by_key(|c| c.as_str());
            if let Some(&category) = categories.get(i) {
                self.selected_category = Some(*category);
                self.state = AppState::CategoryView;
                self.status_message = format!("{} · {}", category.as_str(), category.description());
            }
        }
    }

    fn confirm_deletion(&mut self) {
        self.state = AppState::Deleting;
        self.status_message = "Deleting...".to_string();
        
        if let Some(ref mut result) = self.scan_result {
            let to_delete: Vec<(usize, std::path::PathBuf)> = self.marked_for_deletion.iter()
                .filter_map(|&i| result.entries.get(i).map(|e| (i, e)))
                .filter(|(_, e)| !e.is_system)
                .map(|(i, e)| (i, e.path.clone()))
                .collect();

            let paths: Vec<_> = to_delete.iter().map(|(_, p)| p.as_path()).collect();
            
            if paths.is_empty() {
                self.status_message = "No deletable items".to_string();
                self.state = AppState::Viewing;
                return;
            }

            match Cleaner::delete_files(&paths) {
                Ok(results) => {
                    let success_count = results.iter().filter(|(_, success)| *success).count();
                    let failed_count = results.len() - success_count;
                    
                    let deleted_indices: Vec<usize> = results.iter()
                        .zip(to_delete.iter())
                        .filter(|((_, success), _)| *success)
                        .map(|(_, (idx, _))| *idx)
                        .collect();
                    
                    let mut indices_to_remove: Vec<usize> = deleted_indices;
                    indices_to_remove.sort_by(|a, b| b.cmp(a));
                    
                    for idx in indices_to_remove {
                        if idx < result.entries.len() {
                            let removed = result.entries.remove(idx);
                            result.total_size = result.total_size.saturating_sub(removed.size);
                            if removed.is_dir {
                                result.total_dirs = result.total_dirs.saturating_sub(1);
                            } else {
                                result.total_files = result.total_files.saturating_sub(1);
                            }
                        }
                    }
                    
                    self.categories = Analyzer::group_by_category(&result.entries);
                    self.recommendations = Analyzer::get_recommendations(&result.entries);
                    self.marked_for_deletion.clear();
                    
                    // Update storage info
                    self.storage_info = StorageInfo::from_path(&self.scan_path);
                    
                    if result.entries.is_empty() {
                        self.list_state.select(None);
                    } else if let Some(selected) = self.list_state.selected() {
                        if selected >= result.entries.len() {
                            self.list_state.select(Some(result.entries.len().saturating_sub(1)));
                        }
                    }
                    
                    if failed_count > 0 {
                        self.status_message = format!("✓ Deleted {} · ✗ {} failed", success_count, failed_count);
                    } else {
                        self.status_message = format!("✓ Deleted {} items", success_count);
                    }
                }
                Err(e) => {
                    self.status_message = format!("✗ Error: {}", e);
                }
            }
        }
        self.state = AppState::Viewing;
    }

    fn switch_view(&mut self) {
        self.current_view = match self.current_view {
            ViewMode::AllFiles => ViewMode::Categories,
            ViewMode::Categories => ViewMode::AllFiles,
        };
        self.status_message = match self.current_view {
            ViewMode::AllFiles => "File Browser".to_string(),
            ViewMode::Categories => "Categories · Enter to drill down".to_string(),
        };
    }

    fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.status_message = if self.show_hidden {
            "Showing hidden files".to_string()
        } else {
            "Hidden files filtered".to_string()
        };
    }
    
    fn get_scan_path_from_option(&self) -> PathBuf {
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
    
    fn update_path_suggestions(&mut self) {
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

pub async fn run_app(scan_path: PathBuf, min_size: u64, depth: usize) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(scan_path.clone());
    app.home_menu.min_size_mb = min_size;
    app.home_menu.max_depth = depth;
    
    // Main event loop starting with home screen
    let result = run_main_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn run_main_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        app.frame_count = app.frame_count.wrapping_add(1);
        
        // Render based on state
        terminal.draw(|f| render_ui(f, app))?;
        
        // Handle events with timeout for animations
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                
                match app.state.clone() {
                    AppState::Home => {
                        handle_home_input(app, key.code)?;
                        
                        // Check if we should start scanning
                        if app.state == AppState::Scanning {
                            run_scan(terminal, app).await?;
                        }
                    }
                    AppState::PathInput => {
                        handle_path_input(app, key.code);
                    }
                    AppState::Scanning => {
                        // Scanning is handled by run_scan
                        if key.code == KeyCode::Char('q') {
                            app.state = AppState::Home;
                        }
                    }
                    AppState::ScanComplete => {
                        handle_scan_complete_input(app, key.code)?;
                    }
                    AppState::SystemWarning => {
                        handle_system_warning_input(app, key.code);
                    }
                    AppState::Confirmation => {
                        handle_confirmation_input(app, key.code);
                    }
                    _ => {
                        if handle_viewing_input(app, key.code)? {
                            return Ok(());  // User quit
                        }
                    }
                }
            }
        }
        
        // Yield to let async tasks run
        tokio::task::yield_now().await;
    }
}

fn handle_home_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') => std::process::exit(0),
        KeyCode::Up | KeyCode::Char('k') => {
            if app.home_menu.selected_option > 0 {
                app.home_menu.selected_option -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.home_menu.selected_option < app.home_menu.options.len() - 1 {
                app.home_menu.selected_option += 1;
            }
        }
        KeyCode::Enter => {
            let selected = &app.home_menu.options[app.home_menu.selected_option];
            if matches!(selected, ScanOption::CustomPath) && app.home_menu.custom_path.is_empty() {
                app.state = AppState::PathInput;
                app.path_input.clear();
                app.update_path_suggestions();
            } else {
                app.scan_path = app.get_scan_path_from_option();
                app.current_path = app.scan_path.clone();
                app.state = AppState::Scanning;
            }
        }
        KeyCode::Char('p') => {
            app.state = AppState::PathInput;
            app.path_input = app.home_menu.custom_path.clone();
            app.path_cursor = app.path_input.len();
            app.update_path_suggestions();
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.home_menu.min_size_mb = (app.home_menu.min_size_mb + 1).min(1000);
        }
        KeyCode::Char('-') => {
            app.home_menu.min_size_mb = app.home_menu.min_size_mb.saturating_sub(1).max(1);
        }
        KeyCode::Char('d') => {
            app.home_menu.max_depth = if app.home_menu.max_depth == 0 { 5 } else { (app.home_menu.max_depth + 1) % 11 };
        }
        KeyCode::Char('.') => {
            app.home_menu.include_hidden = !app.home_menu.include_hidden;
        }
        _ => {}
    }
    Ok(())
}

fn handle_path_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.state = AppState::Home;
        }
        KeyCode::Enter => {
            if !app.path_input.is_empty() && PathBuf::from(&app.path_input).exists() {
                app.home_menu.custom_path = app.path_input.clone();
                app.home_menu.selected_option = 2; // Custom path option
            }
            app.state = AppState::Home;
        }
        KeyCode::Tab => {
            // Auto-complete with first suggestion
            if !app.home_menu.path_suggestions.is_empty() {
                app.path_input = app.home_menu.path_suggestions[0].clone();
                app.path_cursor = app.path_input.len();
            }
            app.update_path_suggestions();
        }
        KeyCode::Backspace => {
            if app.path_cursor > 0 {
                app.path_input.remove(app.path_cursor - 1);
                app.path_cursor -= 1;
            }
            app.update_path_suggestions();
        }
        KeyCode::Delete => {
            if app.path_cursor < app.path_input.len() {
                app.path_input.remove(app.path_cursor);
            }
            app.update_path_suggestions();
        }
        KeyCode::Left => {
            app.path_cursor = app.path_cursor.saturating_sub(1);
        }
        KeyCode::Right => {
            app.path_cursor = (app.path_cursor + 1).min(app.path_input.len());
        }
        KeyCode::Char(c) => {
            app.path_input.insert(app.path_cursor, c);
            app.path_cursor += 1;
            app.update_path_suggestions();
        }
        _ => {}
    }
}

fn handle_system_warning_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            for idx in &app.pending_system_deletions {
                if let Some(pos) = app.marked_for_deletion.iter().position(|&x| x == *idx) {
                    app.marked_for_deletion.remove(pos);
                }
            }
            app.pending_system_deletions.clear();
            app.status_message = "System files unmarked".to_string();
            if !app.marked_for_deletion.is_empty() {
                app.delete_marked();
            } else {
                app.state = AppState::Viewing;
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            for idx in &app.pending_system_deletions {
                if let Some(pos) = app.marked_for_deletion.iter().position(|&x| x == *idx) {
                    app.marked_for_deletion.remove(pos);
                }
            }
            app.pending_system_deletions.clear();
            app.state = AppState::Viewing;
            app.status_message = "Cancelled".to_string();
        }
        _ => {}
    }
}

fn handle_confirmation_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_deletion();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.state = AppState::Viewing;
            app.status_message = "Cancelled".to_string();
        }
        _ => {}
    }
}

fn handle_scan_complete_input(app: &mut App, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Enter => {
            // Go to file browser view
            app.state = AppState::Viewing;
        }
        KeyCode::Char('s') => {
            // Select all safe items and go to view
            if let Some(ref result) = app.scan_result {
                app.marked_for_deletion = result.entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        let cat = Analyzer::categorize_file(e);
                        cat.is_safe_to_delete() && !e.is_system
                    })
                    .map(|(i, _)| i)
                    .collect();
                let size: u64 = app.marked_for_deletion.iter()
                    .filter_map(|&i| result.entries.get(i))
                    .map(|e| e.size)
                    .sum();
                app.status_message = format!(
                    "✓ {} safe items selected · {}",
                    app.marked_for_deletion.len(),
                    humansize::format_size(size, humansize::DECIMAL)
                );
            }
            app.state = AppState::Viewing;
        }
        KeyCode::Char('h') => {
            // Go back to home
            app.state = AppState::Home;
            app.scan_result = None;
            app.marked_for_deletion.clear();
            app.navigation_stack.clear();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_viewing_input(app: &mut App, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Down | KeyCode::Char('j') => app.next_item(),
        KeyCode::Up | KeyCode::Char('k') => app.previous_item(),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
            if app.current_view == ViewMode::AllFiles {
                app.enter_folder();
            } else if app.current_view == ViewMode::Categories {
                app.enter_category_view();
            }
        }
        KeyCode::Left | KeyCode::Backspace => {
            if app.state == AppState::CategoryView {
                app.state = AppState::Viewing;
                app.selected_category = None;
                app.status_message = "Back".to_string();
            } else {
                app.go_back();
            }
        }
        KeyCode::Char(' ') => app.toggle_mark(),
        KeyCode::Char('d') => app.delete_marked(),
        KeyCode::Char('?') => app.show_help = !app.show_help,
        KeyCode::Char('v') => app.switch_view(),
        KeyCode::Char('.') => app.toggle_hidden(),
        KeyCode::Char('a') => {
            if app.current_view == ViewMode::AllFiles {
                if let Some(ref result) = app.scan_result {
                    app.marked_for_deletion = result.entries
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| !e.is_system)
                        .map(|(i, _)| i)
                        .collect();
                    app.status_message = format!("{} items marked", app.marked_for_deletion.len());
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(ref result) = app.scan_result {
                app.marked_for_deletion = result.entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        let cat = Analyzer::categorize_file(e);
                        cat.is_safe_to_delete() && !e.is_system
                    })
                    .map(|(i, _)| i)
                    .collect();
                let size: u64 = app.marked_for_deletion.iter()
                    .filter_map(|&i| result.entries.get(i))
                    .map(|e| e.size)
                    .sum();
                app.status_message = format!(
                    "✓ {} safe items · {}",
                    app.marked_for_deletion.len(),
                    humansize::format_size(size, humansize::DECIMAL)
                );
            }
        }
        KeyCode::Char('c') => {
            app.marked_for_deletion.clear();
            app.status_message = "Selection cleared".to_string();
        }
        KeyCode::Char('h') => {
            // Go back to home
            app.state = AppState::Home;
            app.scan_result = None;
            app.marked_for_deletion.clear();
            app.navigation_stack.clear();
        }
        KeyCode::Esc => {
            if app.show_help {
                app.show_help = false;
            } else if app.state == AppState::CategoryView {
                app.state = AppState::Viewing;
                app.selected_category = None;
            } else if !app.navigation_stack.is_empty() {
                app.go_back();
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn run_scan(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let progress = app.scan_progress.clone();
    let min_size = app.home_menu.min_size_mb;
    let depth = app.home_menu.max_depth;
    
    let scanner = Scanner::new(min_size, depth);
    let scan_path_clone = app.scan_path.clone();
    let progress_clone = progress.clone();
    
    eprintln!("🔍 Starting scan of: {}", app.scan_path.display());
    
    let scan_handle = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(scanner.scan_with_progress(&scan_path_clone, progress_clone))
    });

    // Scanning loop
    let mut _last_update = std::time::Instant::now();
    let mut last_files_count = 0;
    
    loop {
        app.frame_count = app.frame_count.wrapping_add(1);
        
        // Update progress snapshot
        if app.frame_count % 3 == 0 {
            if let Ok(prog) = app.scan_progress.try_lock() {
                app.last_progress_snapshot = ScanProgressSnapshot {
                    current_path: prog.current_path.clone(),
                    files_scanned: prog.files_scanned,
                    dirs_scanned: prog.dirs_scanned,
                    total_size_scanned: prog.total_size_scanned,
                    entries_count: prog.entries.len(),
                    top_entries: prog.entries.iter()
                        .rev()  // Show most recent first
                        .take(50)
                        .map(|e| (e.name.clone(), e.size, Analyzer::categorize_file(e).as_str().to_string()))
                        .collect(),
                };
                
                if prog.is_complete {
                    break;
                }
                
                let current_files = prog.files_scanned;
                if current_files > last_files_count {
                    _last_update = std::time::Instant::now();
                    last_files_count = current_files;
                }
            }
        }

        terminal.draw(|f| {
            f.render_widget(Clear, f.area());
            render_scanning_enhanced(f, app, app.frame_count, app.scan_scroll_offset);
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.state = AppState::Home;
                            return Ok(());
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.scan_scroll_offset = app.scan_scroll_offset.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max_scroll = app.last_progress_snapshot.top_entries.len().saturating_sub(5);
                            app.scan_scroll_offset = (app.scan_scroll_offset + 1).min(max_scroll);
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if scan_handle.is_finished() {
            break;
        }
        
        tokio::task::yield_now().await;
    }

    // Process results
    match scan_handle.await {
        Ok(Ok(result)) => {
            eprintln!("✓ Scan successful: {} files, {} dirs",
                result.total_files, result.total_dirs);
            
            app.recommendations = Analyzer::get_recommendations(&result.entries);
            app.categories = Analyzer::group_by_category(&result.entries);
            
            let safe_savings = Analyzer::calculate_safe_savings(&result.entries);
            app.status_message = format!(
                "Scan complete · {} potential savings",
                humansize::format_size(safe_savings, humansize::DECIMAL)
            );
            
            app.scan_result = Some(result);
            app.state = AppState::ScanComplete;  // Show summary first
            app.list_state.select(Some(0));
            app.category_state.select(Some(0));
        }
        Ok(Err(e)) => {
            app.status_message = format!("Scan failed: {}", e);
            app.state = AppState::Home;
        }
        Err(e) => {
            app.status_message = format!("Scan error: {}", e);
            app.state = AppState::Home;
        }
    }

    Ok(())
}

fn render_ui(f: &mut Frame, app: &mut App) {
    f.render_widget(Clear, f.area());
    
    match app.state {
        AppState::Home => {
            render_home(f, &app.home_menu, app.frame_count);
        }
        AppState::PathInput => {
            render_home(f, &app.home_menu, app.frame_count);
            render_path_input(f, &app.path_input, app.path_cursor, &app.home_menu.path_suggestions);
        }
        AppState::Scanning => {
            render_scanning_enhanced(f, app, app.frame_count, app.scan_scroll_offset);
        }
        AppState::ScanComplete => {
            render_scan_complete(f, app, f.area());
        }
        AppState::SystemWarning => {
            render_results_view(f, app, f.area());
            render_system_warning_dialog(f, &app.system_warning_message, f.area());
        }
        AppState::Confirmation => {
            render_results_view(f, app, f.area());
            render_confirmation_dialog(f, &app.status_message, f.area());
        }
        _ => {
            render_results_view(f, app, f.area());
            if app.show_help {
                render_help_overlay(f, f.area());
            }
        }
    }
}

