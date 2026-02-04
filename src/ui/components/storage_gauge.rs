#![allow(dead_code)]

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Gauge},
};
use crate::ui::colors::*;
use crate::ui::types::StorageInfo;

/// Render storage overview gauge
pub fn render_storage_overview(f: &mut Frame, storage: &StorageInfo, area: Rect) {
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { ACCENT };
    
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    let free = humansize::format_size(storage.available_space, humansize::DECIMAL);
    
    let label = format!("  💾 {} used of {} ({:.0}%)  ·  {} free  ", used, total, usage * 100.0, free);
    
    let gauge = Gauge::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75)))
            .title(Span::styled(" System Storage ", Style::default().fg(TEXT_DIM))))
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(35, 35, 50)))
        .ratio(usage.min(1.0))
        .label(Span::styled(label, Style::default().fg(TEXT)));
    
    f.render_widget(gauge, area);
}

/// Render compact storage gauge (no title)
pub fn render_compact_storage_gauge(f: &mut Frame, storage: &StorageInfo, area: Rect) {
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { SUCCESS };
    
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    let free = humansize::format_size(storage.available_space, humansize::DECIMAL);
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(35, 35, 50)))
        .ratio(usage.min(1.0))
        .label(Span::styled(
            format!("{} / {} ({:.0}%) · {} free", used, total, usage * 100.0, free),
            Style::default().fg(TEXT)
        ));
    f.render_widget(gauge, area);
}

/// Render storage with deletion info
pub fn render_storage_with_selection(f: &mut Frame, storage: &StorageInfo, selected_size: u64, area: Rect) {
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { SUCCESS };
    
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    let selected = humansize::format_size(selected_size, humansize::DECIMAL);
    
    let label = if selected_size > 0 {
        format!("{} / {} · {} selected for deletion", used, total, selected)
    } else {
        format!("{} / {} ({:.0}% used)", used, total, usage * 100.0)
    };
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(35, 35, 50)))
        .ratio(usage.min(1.0))
        .label(Span::styled(label, Style::default().fg(TEXT)));
    f.render_widget(gauge, area);
}
