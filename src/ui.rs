use crate::analyzer::{Analyzer, FileCategory};
use crate::cleaner::Cleaner;
use crate::scanner::{FileEntry, ScanProgress, ScanResult, Scanner};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Wrap, Gauge, Clear,
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// Modern color palette
const ACCENT: Color = Color::Rgb(99, 102, 241);     // Indigo
const SUCCESS: Color = Color::Rgb(34, 197, 94);     // Green
const WARNING: Color = Color::Rgb(251, 191, 36);    // Amber
const DANGER: Color = Color::Rgb(239, 68, 68);      // Red
const MUTED: Color = Color::Rgb(107, 114, 128);     // Gray
const TEXT: Color = Color::Rgb(226, 232, 240);      // Light text
const TEXT_DIM: Color = Color::Rgb(148, 163, 184);  // Dimmed text

#[derive(PartialEq, Clone)]
enum AppState {
    Scanning,
    Viewing,
    CategoryView,
    Deleting,
    Confirmation,
    SystemWarning,
}

pub struct App {
    state: AppState,
    scan_result: Option<ScanResult>,
    scan_path: PathBuf,
    current_path: PathBuf,           // Current browsing path
    navigation_stack: Vec<PathBuf>,  // For back navigation
    list_state: ListState,
    category_state: ListState,
    selected_category: Option<FileCategory>,
    categories: HashMap<FileCategory, Vec<FileEntry>>,
    marked_for_deletion: Vec<usize>,
    recommendations: Vec<String>,
    status_message: String,
    show_help: bool,
    current_view: ViewMode,
    scan_progress: Arc<Mutex<ScanProgress>>,
    system_warning_message: String,
    pending_system_deletions: Vec<usize>,
    show_hidden: bool,
    last_progress_snapshot: ScanProgressSnapshot,  // Cache for smooth rendering
    storage_info: StorageInfo,
}

#[derive(Clone, Default)]
struct ScanProgressSnapshot {
    current_path: String,
    files_scanned: usize,
    dirs_scanned: usize,
    total_size_scanned: u64,
    entries_count: usize,
    top_entries: Vec<(String, u64, String)>,  // (name, size, category)
}

#[derive(Clone, Default)]
struct StorageInfo {
    total_space: u64,
    available_space: u64,
    used_space: u64,
}

impl StorageInfo {
    fn from_path(path: &std::path::Path) -> Self {
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
    
    fn usage_percent(&self) -> f64 {
        if self.total_space == 0 {
            0.0
        } else {
            self.used_space as f64 / self.total_space as f64
        }
    }
}

#[derive(PartialEq, Clone)]
enum ViewMode {
    AllFiles,
    Categories,
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

    fn get_current_entries(&self) -> Vec<(usize, &FileEntry)> {
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
                    Scanner::get_system_warning(&system_entry.path)
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
    
    let scan_handle = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(scanner.scan_with_progress(&scan_path_clone, progress_clone))
    });

    // Scanning loop with smooth rendering
    let mut frame_count = 0u32;
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
                
                if prog.is_complete {
                    break;
                }
            }
        }
        frame_count = frame_count.wrapping_add(1);

        terminal.draw(|f| render_scanning(f, &app, frame_count))?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                    terminal.show_cursor()?;
                    return Ok(());
                }
            }
        }
        
        tokio::task::yield_now().await;
    }

    // Get scan results
    match scan_handle.await {
        Ok(Ok(result)) => {
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
            app.status_message = format!("Scan failed: {}", e);
            app.state = AppState::Viewing;
        }
        Err(e) => {
            app.status_message = format!("Scan error: {}", e);
            app.state = AppState::Viewing;
        }
    }

    let result = run_ui(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn render_scanning(f: &mut Frame, app: &App, frame_count: u32) {
    let area = f.area();
    
    // Clear with dark background
    f.render_widget(Clear, area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(5),  // Storage info
            Constraint::Length(4),  // Progress
            Constraint::Length(3),  // Current file
            Constraint::Min(8),     // Files found
            Constraint::Length(2),  // Footer
        ])
        .split(area);

    // Header - clean, no borders
    let header = Paragraph::new(Line::from(vec![
        Span::styled("◉ ", Style::default().fg(ACCENT)),
        Span::styled("DISK CLEANER", Style::default().fg(TEXT).bold()),
        Span::styled(" · Scanning", Style::default().fg(TEXT_DIM)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Storage info bar
    render_storage_bar(f, &app.storage_info, chunks[1]);

    // Progress section
    let snap = &app.last_progress_snapshot;
    let progress_text = format!(
        "{}  files   {}  dirs   {}  found",
        snap.files_scanned,
        snap.dirs_scanned,
        humansize::format_size(snap.total_size_scanned, humansize::DECIMAL)
    );
    
    // Animated progress bar
    let animation_pos = (frame_count % 40) as f64 / 40.0;
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(ACCENT).bg(Color::Rgb(45, 45, 60)))
        .ratio(animation_pos)
        .label(Span::styled(progress_text, Style::default().fg(TEXT)));
    f.render_widget(gauge, chunks[2]);

    // Current path being scanned
    let current_display = if snap.current_path.len() > 60 {
        format!("...{}", &snap.current_path[snap.current_path.len()-57..])
    } else {
        snap.current_path.clone()
    };
    let current = Paragraph::new(Line::from(vec![
        Span::styled("→ ", Style::default().fg(MUTED)),
        Span::styled(current_display, Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(current, chunks[3]);

    // Files found list
    let items: Vec<ListItem> = snap.top_entries
        .iter()
        .map(|(name, size, cat)| {
            let display_name = if name.len() > 35 {
                format!("{}...", &name[..32])
            } else {
                name.clone()
            };
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("{:<36}", display_name), Style::default().fg(TEXT)),
                Span::styled(format!("{:>10}", humansize::format_size(*size, humansize::DECIMAL)), Style::default().fg(ACCENT)),
                Span::styled(format!("  {}", cat), Style::default().fg(TEXT_DIM)),
            ]))
        })
        .collect();

    let list_title = format!("  Found {} items", snap.entries_count);
    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(list_title, Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE));
    f.render_widget(list, chunks[4]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Press ", Style::default().fg(MUTED)),
        Span::styled("Q", Style::default().fg(ACCENT)),
        Span::styled(" to cancel", Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[5]);
}

fn render_storage_bar(f: &mut Frame, storage: &StorageInfo, area: Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    // Storage text info
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    let available = humansize::format_size(storage.available_space, humansize::DECIMAL);
    
    let info = Paragraph::new(Line::from(vec![
        Span::styled("💾 ", Style::default()),
        Span::styled(format!("{} used", used), Style::default().fg(TEXT)),
        Span::styled(" of ", Style::default().fg(MUTED)),
        Span::styled(total, Style::default().fg(TEXT)),
        Span::styled("  ·  ", Style::default().fg(MUTED)),
        Span::styled(format!("{} free", available), Style::default().fg(SUCCESS)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(info, inner[0]);

    // Storage bar
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 {
        DANGER
    } else if usage > 0.75 {
        WARNING
    } else {
        SUCCESS
    };
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(45, 45, 60)))
        .ratio(usage.min(1.0))
        .label(format!("{:.1}%", usage * 100.0));
    f.render_widget(gauge, inner[1]);
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

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    // Title and stats
    let stats = if let Some(ref result) = app.scan_result {
        format!(
            "📄 {} files  📁 {} dirs  💾 {}  ✓ {} marked",
            result.total_files,
            result.total_dirs,
            humansize::format_size(result.total_size, humansize::DECIMAL),
            app.marked_for_deletion.len()
        )
    } else {
        String::new()
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("◉ ", Style::default().fg(ACCENT)),
        Span::styled("DISK CLEANER", Style::default().fg(TEXT).bold()),
        Span::styled("  ", Style::default()),
        Span::styled(stats, Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(header, chunks[0]);

    // Compact storage bar
    let storage = &app.storage_info;
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { SUCCESS };
    
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(45, 45, 60)))
        .ratio(usage.min(1.0))
        .label(Span::styled(
            format!("{} / {} ({:.0}%)", used, total, usage * 100.0),
            Style::default().fg(TEXT)
        ));
    f.render_widget(gauge, chunks[1]);
}

fn render_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    let path_display = app.current_path.to_string_lossy();
    let truncated = if path_display.len() > 80 {
        format!("...{}", &path_display[path_display.len()-77..])
    } else {
        path_display.to_string()
    };

    let breadcrumb = Paragraph::new(Line::from(vec![
        Span::styled("📂 ", Style::default()),
        Span::styled(truncated, Style::default().fg(ACCENT)),
        if !app.navigation_stack.is_empty() {
            Span::styled("  ← Backspace to go back", Style::default().fg(MUTED))
        } else {
            Span::raw("")
        },
    ]));
    f.render_widget(breadcrumb, area);
}

fn render_system_warning(f: &mut Frame, app: &App, area: Rect) {
    let warning_lines: Vec<Line> = app.system_warning_message
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(DANGER))))
        .collect();

    let warning = Paragraph::new(warning_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DANGER))
            .title(Span::styled(" ⚠️  DANGER ", Style::default().fg(DANGER).bold())))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    
    f.render_widget(warning, area);
}

fn render_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let current_entries = app.get_current_entries();
    
    let items: Vec<ListItem> = current_entries
        .iter()
        .map(|(actual_idx, entry)| {
            let category = Analyzer::categorize_file(entry);
            let marked = if app.marked_for_deletion.contains(actual_idx) {
                Span::styled("● ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("○ ", Style::default().fg(MUTED))
            };
            
            let icon = if entry.is_system {
                Span::styled("⚙ ", Style::default().fg(DANGER))
            } else if entry.is_hidden {
                Span::styled("◌ ", Style::default().fg(MUTED))
            } else if entry.is_dir {
                Span::styled("▸ ", Style::default().fg(ACCENT))
            } else {
                Span::styled("  ", Style::default())
            };
            
            let name_style = if entry.is_system {
                Style::default().fg(DANGER).dim()
            } else if entry.is_dir {
                Style::default().fg(TEXT).bold()
            } else {
                Style::default().fg(TEXT)
            };

            let name_display = if entry.name.len() > 30 {
                format!("{}...", &entry.name[..27])
            } else {
                format!("{:<30}", entry.name)
            };
            
            let size_str = humansize::format_size(entry.size, humansize::DECIMAL);
            
            ListItem::new(Line::from(vec![
                marked,
                icon,
                Span::styled(name_display, name_style),
                Span::styled(format!("{:>10}", size_str), Style::default().fg(TEXT_DIM)),
                Span::styled(format!("  {}", category.as_str()), Style::default().fg(category.color())),
            ]))
        })
        .collect();

    let hidden_indicator = if app.show_hidden { "" } else { " (hidden filtered)" };
    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(
                format!(" Files{} ", hidden_indicator),
                Style::default().fg(TEXT_DIM)
            ))
            .borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::Rgb(55, 55, 75)))
        .highlight_symbol("  ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_category_view(f: &mut Frame, app: &mut App, area: Rect) {
    let mut categories: Vec<_> = app.categories.iter().collect();
    categories.sort_by(|a, b| {
        let size_a: u64 = a.1.iter().map(|e| e.size).sum();
        let size_b: u64 = b.1.iter().map(|e| e.size).sum();
        size_b.cmp(&size_a)
    });

    let items: Vec<ListItem> = categories
        .iter()
        .map(|(category, entries)| {
            let total_size: u64 = entries.iter().map(|e| e.size).sum();
            let safe_indicator = if category.is_safe_to_delete() {
                Span::styled("✓ ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("! ", Style::default().fg(WARNING))
            };
            
            ListItem::new(Line::from(vec![
                safe_indicator,
                Span::styled(format!("{:<20}", category.as_str()), Style::default().fg(category.color())),
                Span::styled(format!("{:>6} items", entries.len()), Style::default().fg(TEXT_DIM)),
                Span::styled(format!("{:>12}", humansize::format_size(total_size, humansize::DECIMAL)), Style::default().fg(TEXT)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(" Categories ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::Rgb(55, 55, 75)))
        .highlight_symbol("  ");

    f.render_stateful_widget(list, area, &mut app.category_state);
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Recommendations
    let rec_items: Vec<Line> = app
        .recommendations
        .iter()
        .take(5)
        .map(|r| Line::from(vec![
            Span::styled("  → ", Style::default().fg(WARNING)),
            Span::styled(r, Style::default().fg(TEXT_DIM)),
        ]))
        .collect();

    let recommendations = Paragraph::new(rec_items)
        .block(Block::default()
            .title(Span::styled(" 💡 Tips ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    
    f.render_widget(recommendations, chunks[0]);

    // Details panel
    let details = if let Some(visible_idx) = app.list_state.selected() {
        let current_entries = app.get_current_entries();
        if let Some((_, entry)) = current_entries.get(visible_idx) {
            let category = Analyzer::categorize_file(entry);
            let cat_str = category.as_str().to_string();
            let cat_color = category.color();
            let is_safe = category.is_safe_to_delete();
            let name = entry.name.clone();
            let size = entry.size;
            let is_dir = entry.is_dir;
            let is_system = entry.is_system;
            let modified = entry.modified.format("%Y-%m-%d").to_string();
            
            let mut lines = vec![
                Line::from(Span::styled(name, Style::default().fg(TEXT).bold())),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Size     ", Style::default().fg(MUTED)),
                    Span::styled(humansize::format_size(size, humansize::DECIMAL), Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Type     ", Style::default().fg(MUTED)),
                    Span::styled(if is_dir { "Directory" } else { "File" }, Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Category ", Style::default().fg(MUTED)),
                    Span::styled(cat_str, Style::default().fg(cat_color)),
                ]),
                Line::from(vec![
                    Span::styled("Modified ", Style::default().fg(MUTED)),
                    Span::styled(modified, Style::default().fg(TEXT)),
                ]),
            ];

            if is_system {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("⚠ System file - protected", Style::default().fg(DANGER))));
            }

            if is_safe {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("✓ Safe to delete", Style::default().fg(SUCCESS))));
            }

            lines
        } else {
            vec![Line::from(Span::styled("No selection", Style::default().fg(MUTED)))]
        }
    } else {
        vec![Line::from(Span::styled("No selection", Style::default().fg(MUTED)))]
    };

    let details_widget = Paragraph::new(details)
        .block(Block::default()
            .title(Span::styled(" Details ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    
    f.render_widget(details_widget, chunks[1]);
}

fn render_category_detail_view(f: &mut Frame, app: &App, area: Rect) {
    if let Some(category) = app.selected_category {
        if let Some(entries) = app.categories.get(&category) {
            let items: Vec<ListItem> = entries
                .iter()
                .map(|entry| {
                    let icon = if entry.is_system {
                        Span::styled("⚙ ", Style::default().fg(DANGER))
                    } else if entry.is_dir {
                        Span::styled("▸ ", Style::default().fg(ACCENT))
                    } else {
                        Span::styled("  ", Style::default())
                    };
                    
                    let name_display = if entry.name.len() > 40 {
                        format!("{}...", &entry.name[..37])
                    } else {
                        entry.name.clone()
                    };
                    
                    ListItem::new(Line::from(vec![
                        icon,
                        Span::styled(format!("{:<42}", name_display), Style::default().fg(TEXT)),
                        Span::styled(
                            humansize::format_size(entry.size, humansize::DECIMAL),
                            Style::default().fg(TEXT_DIM)
                        ),
                    ]))
                })
                .collect();

            let total_size: u64 = entries.iter().map(|e| e.size).sum();
            let safe_text = if category.is_safe_to_delete() { "✓ Safe" } else { "! Review" };
            
            let list = List::new(items)
                .block(Block::default()
                    .title(Span::styled(
                        format!(" {} · {} · {} ", category.as_str(), humansize::format_size(total_size, humansize::DECIMAL), safe_text),
                        Style::default().fg(category.color())
                    ))
                    .borders(Borders::NONE))
                .highlight_style(Style::default().bg(Color::Rgb(55, 55, 75)));

            f.render_widget(list, area);
        }
    }
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled("NAVIGATION", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑ ↓ j k    ", Style::default().fg(TEXT)),
            Span::styled("Move selection", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  → l Enter  ", Style::default().fg(TEXT)),
            Span::styled("Enter folder / View category", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ← Back     ", Style::default().fg(TEXT)),
            Span::styled("Go back", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("ACTIONS", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Space      ", Style::default().fg(TEXT)),
            Span::styled("Toggle mark", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  s          ", Style::default().fg(TEXT)),
            Span::styled("Mark safe items", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  a          ", Style::default().fg(TEXT)),
            Span::styled("Mark all (except system)", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  c          ", Style::default().fg(TEXT)),
            Span::styled("Clear marks", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  d          ", Style::default().fg(TEXT)),
            Span::styled("Delete marked", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("VIEW", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  v          ", Style::default().fg(TEXT)),
            Span::styled("Toggle file/category view", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  .          ", Style::default().fg(TEXT)),
            Span::styled("Toggle hidden files", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", Style::default().fg(TEXT)),
            Span::styled("Toggle help", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  q          ", Style::default().fg(TEXT)),
            Span::styled("Quit", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("INDICATORS", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(SUCCESS)),
            Span::styled("Safe to delete", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ! ", Style::default().fg(WARNING)),
            Span::styled("Review before deleting", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ⚙ ", Style::default().fg(DANGER)),
            Span::styled("System file - protected", Style::default().fg(MUTED)),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default()
            .title(Span::styled(" Help ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    
    f.render_widget(help, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let style = match app.state {
        AppState::Confirmation | AppState::SystemWarning => Style::default().fg(DANGER),
        AppState::Deleting => Style::default().fg(WARNING),
        _ => Style::default().fg(TEXT_DIM),
    };

    let keyhints = if app.state == AppState::Confirmation || app.state == AppState::SystemWarning {
        vec![]
    } else {
        vec![
            Span::styled("  ?", Style::default().fg(ACCENT)),
            Span::styled(" help  ", Style::default().fg(MUTED)),
            Span::styled("Space", Style::default().fg(ACCENT)),
            Span::styled(" mark  ", Style::default().fg(MUTED)),
            Span::styled("d", Style::default().fg(ACCENT)),
            Span::styled(" delete  ", Style::default().fg(MUTED)),
            Span::styled("v", Style::default().fg(ACCENT)),
            Span::styled(" view  ", Style::default().fg(MUTED)),
            Span::styled("q", Style::default().fg(ACCENT)),
            Span::styled(" quit", Style::default().fg(MUTED)),
        ]
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(&app.status_message, style),
        Span::raw("  "),
    ].into_iter().chain(keyhints).collect::<Vec<_>>()))
    .alignment(Alignment::Left);
    
    f.render_widget(footer, area);
}
