use crate::analyzer::{Analyzer, FileCategory};
use crate::cleaner::Cleaner;
use crate::models::{FileEntry, ScanProgress, ScanResult};
use crate::scanner::{Scanner, get_system_warning};
use super::types::*;
use super::render_scanning::render_scanning;
use super::render_main::*;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
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
    pub last_progress_snapshot: ScanProgressSnapshot,  // Cache for smooth rendering
    pub storage_info: StorageInfo,
}

impl App {
    fn new(scan_path: PathBuf) -> Self {
        let storage_info = StorageInfo::from_path(&scan_path);
        Self {
            state: AppState::Scanning,
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
}

pub async fn run_app(scan_path: PathBuf, min_size: u64, depth: usize) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(scan_path.clone());
    let progress = app.scan_progress.clone();

    let scanner = Scanner::new(min_size, depth);
    let scan_path_clone = scan_path.clone();
    let progress_clone = progress.clone();
    
    eprintln!("🔍 Starting scan of: {}", scan_path.display());
    
    let scan_handle = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(scanner.scan_with_progress(&scan_path_clone, progress_clone))
    });

    // Scanning loop with smooth rendering and timeout protection
    let mut frame_count = 0u32;
    let mut last_update = std::time::Instant::now();
    let mut last_files_count = 0;
    let mut stall_warnings = 0;
    
    loop {
        // Update progress snapshot periodically (not every frame to reduce lock contention)
        if frame_count % 3 == 0 {
            if let Ok(prog) = app.scan_progress.try_lock() {
                app.last_progress_snapshot = ScanProgressSnapshot {
                    current_path: prog.current_path.clone(),
                    files_scanned: prog.files_scanned,
                    dirs_scanned: prog.dirs_scanned,
                    total_size_scanned: prog.total_size_scanned,
                    entries_count: prog.entries.len(),
                    top_entries: prog.entries.iter()
                        .take(10)
                        .map(|e| (e.name.clone(), e.size, Analyzer::categorize_file(e).as_str().to_string()))
                        .collect(),
                };
                
                // Check if scan completed
                if prog.is_complete {
                    eprintln!("✓ Scan marked as complete");
                    break;
                }
                
                // Detect if scan is making progress
                let current_files = prog.files_scanned;
                if current_files > last_files_count {
                    last_update = std::time::Instant::now();
                    last_files_count = current_files;
                    stall_warnings = 0;
                } else if last_update.elapsed() > std::time::Duration::from_secs(30) {
                    stall_warnings += 1;
                    if stall_warnings == 1 {
                        eprintln!("⚠️  Scan appears stalled (no new files in 30s), but continuing...");
                    }
                    last_update = std::time::Instant::now();
                }
            }
        }
        frame_count = frame_count.wrapping_add(1);

        terminal.draw(|f| render_scanning(f, &app, frame_count))?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    eprintln!("🛑 Scan cancelled by user");
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                    terminal.show_cursor()?;
                    return Ok(());
                }
            }
        }
        
        // Check if scan task finished
        if scan_handle.is_finished() {
            eprintln!("✓ Scan task finished");
            break;
        }
        
        tokio::task::yield_now().await;
    }

    // Get scan results with better error handling
    eprintln!("📊 Processing scan results...");
    match scan_handle.await {
        Ok(Ok(result)) => {
            eprintln!("✓ Scan successful: {} files, {} dirs, {} total",
                result.total_files, result.total_dirs, 
                humansize::format_size(result.total_size, humansize::DECIMAL));
            
            app.recommendations = Analyzer::get_recommendations(&result.entries);
            app.categories = Analyzer::group_by_category(&result.entries);
            
            let safe_savings = Analyzer::calculate_safe_savings(&result.entries);
            app.status_message = format!(
                "Scan complete · {} potential savings",
                humansize::format_size(safe_savings, humansize::DECIMAL)
            );
            
            app.scan_result = Some(result);
            app.state = AppState::Viewing;
            app.list_state.select(Some(0));
            app.category_state.select(Some(0));
        }
        Ok(Err(e)) => {
            eprintln!("❌ Scan failed: {}", e);
            app.status_message = format!("Scan failed: {} - Press q to exit", e);
            app.state = AppState::Viewing;
        }
        Err(e) => {
            eprintln!("❌ Scan task error: {}", e);
            app.status_message = format!("Scan error: {} - Press q to exit", e);
            app.state = AppState::Viewing;
        }
    }

    let result = run_ui(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run_ui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.state.clone() {
                AppState::SystemWarning => {
                    match key.code {
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
                AppState::Confirmation => {
                    match key.code {
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
                _ => {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
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
                        KeyCode::Esc => {
                            if app.state == AppState::CategoryView {
                                app.state = AppState::Viewing;
                                app.selected_category = None;
                            } else if !app.navigation_stack.is_empty() {
                                app.go_back();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(Clear, area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),  // Header + storage
            Constraint::Length(2),  // Breadcrumb
            Constraint::Min(10),    // Main content
            Constraint::Length(2),  // Status
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_breadcrumb(f, app, chunks[1]);

    if app.state == AppState::SystemWarning {
        render_system_warning(f, app, chunks[2]);
    } else if app.show_help {
        render_help(f, chunks[2]);
    } else if app.state == AppState::CategoryView {
        render_category_detail_view(f, app, chunks[2]);
    } else {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(chunks[2]);

        match app.current_view {
            ViewMode::AllFiles => render_file_list(f, app, content_chunks[0]),
            ViewMode::Categories => render_category_view(f, app, content_chunks[0]),
        }
        
        render_sidebar(f, app, content_chunks[1]);
    }

    render_footer(f, app, chunks[3]);
}

