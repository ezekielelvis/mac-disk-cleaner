use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use crate::analyzer::Analyzer;
use crate::models::FileEntry;
use crate::ui::colors::*;
use crate::ui::components::{render_all_files_footer, render_search_dialog};
use crate::ui::app::App;

/// State for the All Files view
pub struct AllFilesState {
    pub list_state: ListState,
    pub sort_mode: SortMode,
    pub filter_mode: FilterMode,
    pub search_query: String,
    pub search_active: bool,
    #[allow(dead_code)]
    pub scroll_offset: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortMode {
    SizeDesc,
    SizeAsc,
    NameAsc,
    NameDesc,
    DateDesc,
    DateAsc,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FilterMode {
    All,
    SafeOnly,
    LargeOnly,
    Selected,
}

impl Default for AllFilesState {
    fn default() -> Self {
        Self {
            list_state: ListState::default(),
            sort_mode: SortMode::SizeDesc,
            filter_mode: FilterMode::All,
            search_query: String::new(),
            search_active: false,
            scroll_offset: 0,
        }
    }
}

impl AllFilesState {
    #[allow(dead_code)]
    pub fn next(&mut self, total: usize) {
        if total == 0 { return; }
        let i = match self.list_state.selected() {
            Some(i) => if i >= total - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    #[allow(dead_code)]
    pub fn previous(&mut self, total: usize) {
        if total == 0 { return; }
        let i = match self.list_state.selected() {
            Some(i) => if i == 0 { total - 1 } else { i - 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::SizeDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::NameAsc,
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::DateDesc,
            SortMode::DateDesc => SortMode::DateAsc,
            SortMode::DateAsc => SortMode::SizeDesc,
        };
    }

    pub fn cycle_filter(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::All => FilterMode::SafeOnly,
            FilterMode::SafeOnly => FilterMode::LargeOnly,
            FilterMode::LargeOnly => FilterMode::Selected,
            FilterMode::Selected => FilterMode::All,
        };
    }

    pub fn sort_mode_name(&self) -> &'static str {
        match self.sort_mode {
            SortMode::SizeDesc => "Size ↓",
            SortMode::SizeAsc => "Size ↑",
            SortMode::NameAsc => "Name A-Z",
            SortMode::NameDesc => "Name Z-A",
            SortMode::DateDesc => "Date ↓",
            SortMode::DateAsc => "Date ↑",
        }
    }

    pub fn filter_mode_name(&self) -> &'static str {
        match self.filter_mode {
            FilterMode::All => "All Files",
            FilterMode::SafeOnly => "Safe Only",
            FilterMode::LargeOnly => "Large (>50MB)",
            FilterMode::Selected => "Selected",
        }
    }
}

/// Get filtered and sorted entries from scan result
pub fn get_filtered_entries<'a>(
    entries: &'a [FileEntry],
    state: &AllFilesState,
    marked: &[usize],
) -> Vec<(usize, &'a FileEntry)> {
    let mut result: Vec<(usize, &FileEntry)> = entries
        .iter()
        .enumerate()
        .filter(|(idx, e)| {
            // Apply filter
            let passes_filter = match state.filter_mode {
                FilterMode::All => true,
                FilterMode::SafeOnly => {
                    let cat = Analyzer::categorize_file(e);
                    cat.is_safe_to_delete() && !e.is_system
                }
                FilterMode::LargeOnly => e.size > 50 * 1024 * 1024,
                FilterMode::Selected => marked.contains(idx),
            };
            
            // Apply search
            let passes_search = if state.search_query.is_empty() {
                true
            } else {
                let query = state.search_query.to_lowercase();
                e.name.to_lowercase().contains(&query) ||
                e.path.to_string_lossy().to_lowercase().contains(&query)
            };
            
            passes_filter && passes_search
        })
        .collect();

    // Sort
    match state.sort_mode {
        SortMode::SizeDesc => result.sort_by(|a, b| b.1.size.cmp(&a.1.size)),
        SortMode::SizeAsc => result.sort_by(|a, b| a.1.size.cmp(&b.1.size)),
        SortMode::NameAsc => result.sort_by(|a, b| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase())),
        SortMode::NameDesc => result.sort_by(|a, b| b.1.name.to_lowercase().cmp(&a.1.name.to_lowercase())),
        SortMode::DateDesc => result.sort_by(|a, b| b.1.modified.cmp(&a.1.modified)),
        SortMode::DateAsc => result.sort_by(|a, b| a.1.modified.cmp(&b.1.modified)),
    }

    result
}

/// Render the All Files screen from App struct
pub fn render_all_files_screen(f: &mut Frame, app: &mut App, _area: Rect) {
    // Extract data from app
    let entries = match &app.scan_result {
        Some(result) => &result.entries,
        None => return, // No data to render
    };
    
    // Get storage info from app - use reasonable defaults
    let storage_used = app.scan_result.as_ref().map(|r| r.total_size).unwrap_or(0);
    let storage_total = app.scan_result.as_ref().map(|r| r.total_size * 2).unwrap_or(0); // Estimate
    let storage_available = storage_total.saturating_sub(storage_used);
    
    render_all_files_screen_inner(
        f,
        entries,
        &app.marked_for_deletion,
        &mut app.all_files_state,
        storage_used,
        storage_total,
        storage_available,
    );
}

/// Inner render function for All Files screen
fn render_all_files_screen_inner(
    f: &mut Frame,
    entries: &[FileEntry],
    marked_for_deletion: &[usize],
    state: &mut AllFilesState,
    storage_used: u64,
    storage_total: u64,
    storage_available: u64,
) {
    let area = f.area();
    f.render_widget(Clear, area);
    
    let margin = if area.width < 80 { 1 } else { 1 };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints([
            Constraint::Length(3),   // Header with stats
            Constraint::Length(2),   // Storage gauge
            Constraint::Length(3),   // Toolbar with sort/filter
            Constraint::Min(10),     // File list
            Constraint::Length(2),   // Footer
        ])
        .split(area);

    // Get filtered entries
    let filtered_entries = get_filtered_entries(entries, state, marked_for_deletion);
    let total_filtered = filtered_entries.len();
    let selected_count = marked_for_deletion.len();
    let selected_size: u64 = marked_for_deletion.iter()
        .filter_map(|&i| entries.get(i))
        .map(|e| e.size)
        .sum();

    // Header
    render_all_files_header(f, chunks[0], entries.len(), total_filtered, selected_count, selected_size);
    
    // Storage gauge
    render_storage_bar(f, chunks[1], storage_used, storage_total, storage_available, selected_size);
    
    // Toolbar
    render_toolbar(f, chunks[2], state);
    
    // File list
    render_file_list_view(f, chunks[3], &filtered_entries, marked_for_deletion, state);
    
    // Footer
    render_all_files_footer(f, chunks[4]);
    
    // Search overlay if active
    if state.search_active {
        render_search_dialog(f, &state.search_query, area);
    }
}

fn render_all_files_header(f: &mut Frame, area: Rect, total: usize, filtered: usize, selected: usize, selected_size: u64) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("📄 ", Style::default().fg(ACCENT)),
        Span::styled("ALL FILES", Style::default().fg(TEXT).bold()),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
        Span::styled(format!("{} total", total), Style::default().fg(TEXT_DIM)),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
        Span::styled(format!("{} shown", filtered), Style::default().fg(TEXT)),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
        Span::styled(format!("✓ {} selected", selected), Style::default().fg(SUCCESS)),
        Span::styled(format!(" ({})", humansize::format_size(selected_size, humansize::DECIMAL)), Style::default().fg(SUCCESS)),
    ]))
    .block(Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    
    f.render_widget(header, area);
}

fn render_storage_bar(f: &mut Frame, area: Rect, used: u64, total: u64, available: u64, selected_size: u64) {
    use ratatui::widgets::Gauge;
    
    let usage = if total > 0 { used as f64 / total as f64 } else { 0.0 };
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { SUCCESS };
    
    let used_str = humansize::format_size(used, humansize::DECIMAL);
    let total_str = humansize::format_size(total, humansize::DECIMAL);
    let available_str = humansize::format_size(available, humansize::DECIMAL);
    
    let label = if selected_size > 0 {
        format!("{} / {} · {} free · Deleting {} will free space", 
            used_str, total_str, available_str, 
            humansize::format_size(selected_size, humansize::DECIMAL))
    } else {
        format!("{} / {} ({:.0}% used) · {} free", used_str, total_str, usage * 100.0, available_str)
    };
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(35, 35, 50)))
        .ratio(usage.min(1.0))
        .label(Span::styled(label, Style::default().fg(TEXT)));
    
    f.render_widget(gauge, area);
}

fn render_toolbar(f: &mut Frame, area: Rect, state: &AllFilesState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),  // Sort
            Constraint::Length(20),  // Filter
            Constraint::Min(20),     // Search hint
        ])
        .split(area);

    // Sort indicator
    let sort_widget = Paragraph::new(Line::from(vec![
        Span::styled(" Sort: ", Style::default().fg(MUTED)),
        Span::styled(state.sort_mode_name(), Style::default().fg(ACCENT)),
        Span::styled(" [o]", Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(sort_widget, chunks[0]);

    // Filter indicator
    let filter_widget = Paragraph::new(Line::from(vec![
        Span::styled(" Filter: ", Style::default().fg(MUTED)),
        Span::styled(state.filter_mode_name(), Style::default().fg(ACCENT)),
        Span::styled(" [t]", Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(filter_widget, chunks[1]);

    // Search hint
    let search_text = if state.search_query.is_empty() {
        "Press / to search".to_string()
    } else {
        format!("Search: \"{}\" (Esc to clear)", state.search_query)
    };
    let search_widget = Paragraph::new(Line::from(vec![
        Span::styled(" 🔍 ", Style::default().fg(MUTED)),
        Span::styled(search_text, Style::default().fg(if state.search_query.is_empty() { TEXT_DIM } else { TEXT })),
    ]))
    .alignment(Alignment::Right);
    f.render_widget(search_widget, chunks[2]);
}

fn render_file_list_view(
    f: &mut Frame,
    area: Rect,
    entries: &[(usize, &FileEntry)],
    marked: &[usize],
    state: &mut AllFilesState,
) {
    let items: Vec<ListItem> = entries
        .iter()
        .map(|(actual_idx, entry)| {
            let category = Analyzer::categorize_file(entry);
            
            // Checkbox-style marker
            let is_marked = marked.contains(actual_idx);
            let checkbox = if is_marked {
                Span::styled("[✓] ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("[ ] ", Style::default().fg(Color::Rgb(55, 55, 75)))
            };
            
            // File type icon
            let icon = if entry.is_system {
                Span::styled("⚙ ", Style::default().fg(DANGER))
            } else if entry.is_dir {
                Span::styled("📁 ", Style::default().fg(ACCENT))
            } else {
                Span::styled("📄 ", Style::default().fg(TEXT_DIM))
            };
            
            // Name styling
            let name_style = if entry.is_system {
                Style::default().fg(DANGER)
            } else if is_marked {
                Style::default().fg(SUCCESS)
            } else {
                Style::default().fg(TEXT)
            };

            // Truncate name
            let name_display = if entry.name.len() > 30 {
                format!("{}...", &entry.name[..27])
            } else {
                format!("{:<30}", entry.name)
            };
            
            let size_str = humansize::format_size(entry.size, humansize::DECIMAL);
            let date_str = entry.modified.format("%Y-%m-%d").to_string();
            
            // Safety indicator
            let safety = if category.is_safe_to_delete() && !entry.is_system {
                Span::styled("✓", Style::default().fg(SUCCESS))
            } else if entry.is_system {
                Span::styled("⚠", Style::default().fg(DANGER))
            } else {
                Span::styled("·", Style::default().fg(MUTED))
            };
            
            // Path display
            let path_str = entry.path.to_string_lossy();
            let path_display = if path_str.len() > 45 {
                format!("...{}", &path_str[path_str.len()-42..])
            } else {
                path_str.to_string()
            };
            
            ListItem::new(vec![
                Line::from(vec![
                    checkbox,
                    icon,
                    Span::styled(name_display, name_style.bold()),
                    Span::styled(format!("  {:>10}", size_str), Style::default().fg(ACCENT)),
                    Span::styled(format!("  {}", date_str), Style::default().fg(TEXT_DIM)),
                    Span::styled("  ", Style::default()),
                    safety,
                    Span::styled(format!(" {}", category.as_str()), Style::default().fg(category.color())),
                ]),
                Line::from(vec![
                    Span::styled("      ", Style::default()),
                    Span::styled(path_display, Style::default().fg(Color::Rgb(100, 100, 120))),
                ]),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(
                format!(" Files ({} items) - Space to select, d to delete ", entries.len()),
                Style::default().fg(TEXT)
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)))
        .highlight_symbol(" ▸");

    f.render_stateful_widget(list, area, &mut state.list_state);
    
    // Scrollbar
    if entries.len() > (area.height as usize - 2) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(Color::Rgb(75, 75, 95)));
        
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(entries.len())
            .position(state.list_state.selected().unwrap_or(0));
        
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
