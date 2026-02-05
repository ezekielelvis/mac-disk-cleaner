use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::ui::colors::*;

/// Render a stat card with label and value
pub fn render_stat_card(f: &mut Frame, area: Rect, label: &str, value: &str, color: Color, _frame_count: u32) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // Label
            Constraint::Length(2),  // Value
            Constraint::Min(1),     // Padding
        ])
        .split(area);

    let label_widget = Paragraph::new(label)
        .style(Style::default().fg(MUTED))
        .alignment(Alignment::Center);
    f.render_widget(label_widget, chunks[0]);

    let value_widget = Paragraph::new(value)
        .style(Style::default().fg(color).bold().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(value_widget, chunks[1]);

    // Card border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60)));
    f.render_widget(block, area);
}
/// Render stats panel for scanning view
pub fn render_stats_panel(f: &mut Frame, area: Rect, files_scanned: usize, dirs_scanned: usize, total_size: u64, entries_count: usize, frame_count: u32) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Files scanned card
    render_stat_card(f, chunks[0], "📄 FILES", &format!("{}", files_scanned), ACCENT, frame_count);
    
    // Directories scanned card
    render_stat_card(f, chunks[1], "📁 DIRS", &format!("{}", dirs_scanned), SUCCESS, frame_count);
    
    // Total size card
    let size_str = humansize::format_size(total_size, humansize::DECIMAL);
    render_stat_card(f, chunks[2], "💾 SIZE", &size_str, WARNING, frame_count);
    
    // Items found card
    render_stat_card(f, chunks[3], "🔍 FOUND", &format!("{}", entries_count), Color::Rgb(168, 85, 247), frame_count);
}

/// Render quick tips with rotation
pub fn render_quick_tips(f: &mut Frame, area: Rect, frame_count: u32) {
    let tips = [
        "💡 Use 'Quick Scan' for fast cleanup of temporary files",
        "💡 'Large Files' mode helps find forgotten downloads",
        "💡 System files are protected and cannot be deleted",
        "💡 Press '?' during scan results for keyboard shortcuts",
        "💡 Use category view (v) to see files grouped by type",
        "💡 Use mouse scroll wheel to navigate lists",
    ];
    
    let tip_idx = ((frame_count / 100) % tips.len() as u32) as usize;
    
    let tip = Paragraph::new(Line::from(vec![
        Span::styled(tips[tip_idx], Style::default().fg(TEXT_DIM)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    
    f.render_widget(tip, area);
}
