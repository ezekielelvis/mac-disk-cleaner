use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, Gauge, List, ListItem, Paragraph, Block, Borders},
};
use crate::ui::app::App;
use crate::ui::types::StorageInfo;
use crate::ui::colors::*;

/// Render the scanning view while a scan is in progress
pub fn render_scanning(f: &mut Frame, app: &App, frame_count: u32) {
    let area = f.area();
    
    // Clear with dark background
    f.render_widget(Clear, area);
    
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            ratatui::layout::Constraint::Length(3),  // Header
            ratatui::layout::Constraint::Length(5),  // Storage info
            ratatui::layout::Constraint::Length(4),  // Progress
            ratatui::layout::Constraint::Length(3),  // Current file
            ratatui::layout::Constraint::Min(8),     // Files found
            ratatui::layout::Constraint::Length(2),  // Footer
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

/// Render the storage information bar
pub fn render_storage_bar(f: &mut Frame, storage: &StorageInfo, area: Rect) {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);
    
    // Storage label
    let label = Paragraph::new(Line::from(vec![
        Span::styled("Storage: ", Style::default().fg(TEXT_DIM)),
        Span::styled("System", Style::default().fg(TEXT)),
    ]));
    f.render_widget(label, chunks[0]);
    
    // Storage gauge
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    let usage = storage.usage_percent();
    let label_text = format!("{} / {} ({:.0}% used)", used, total, usage * 100.0);
    
    let color = if usage >= 0.90 {
        Color::Red
    } else if usage >= 0.75 {
        Color::Yellow
    } else {
        ACCENT
    };
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color).bg(Color::Rgb(45, 45, 60)))
        .ratio(usage)
        .label(Span::styled(label_text, Style::default().fg(TEXT)));
    f.render_widget(gauge, chunks[1]);
}
