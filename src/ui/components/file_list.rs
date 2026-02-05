use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use crate::analyzer::Analyzer;
use crate::models::FileEntry;
use crate::ui::colors::*;

/// Render an enhanced file list with icons and indicators
pub fn render_file_list(
    f: &mut Frame, 
    area: Rect, 
    entries: &[(usize, &FileEntry)],
    marked_for_deletion: &[usize],
    show_hidden: bool,
    list_state: &mut ListState,
) {
    let total_items = entries.len();
    
    let items: Vec<ListItem> = entries
        .iter()
        .map(|(actual_idx, entry)| {
            let category = Analyzer::categorize_file(entry);
            
            // Selection marker
            let marked = if marked_for_deletion.contains(actual_idx) {
                Span::styled("● ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("○ ", Style::default().fg(Color::Rgb(55, 55, 75)))
            };
            
            // File type icon
            let icon = if entry.is_system {
                Span::styled("⚙ ", Style::default().fg(DANGER))
            } else if entry.is_hidden {
                Span::styled("◌ ", Style::default().fg(MUTED))
            } else if entry.is_dir {
                Span::styled("▶ ", Style::default().fg(ACCENT))
            } else {
                Span::styled("  ", Style::default())
            };
            
            // Name styling
            let name_style = if entry.is_system {
                Style::default().fg(DANGER).dim()
            } else if entry.is_dir {
                Style::default().fg(TEXT).bold()
            } else if marked_for_deletion.contains(actual_idx) {
                Style::default().fg(SUCCESS)
            } else {
                Style::default().fg(TEXT)
            };

            let name_display = if entry.name.len() > 28 {
                format!("{}...", &entry.name[..25])
            } else {
                format!("{:<28}", entry.name)
            };
            
            let size_str = humansize::format_size(entry.size, humansize::DECIMAL);
            
            // Safety indicator
            let safety = if category.is_safe_to_delete() {
                Span::styled("✓", Style::default().fg(SUCCESS))
            } else if entry.is_system {
                Span::styled("⚠", Style::default().fg(DANGER))
            } else {
                Span::styled("·", Style::default().fg(MUTED))
            };
            
            ListItem::new(Line::from(vec![
                marked,
                icon,
                Span::styled(name_display, name_style),
                Span::styled(format!("{:>9}", size_str), Style::default().fg(TEXT_DIM)),
                Span::styled("  ", Style::default()),
                safety,
                Span::styled(format!(" {}", category.as_str()), Style::default().fg(category.color())),
            ]))
        })
        .collect();

    let hidden_text = if !show_hidden { " (hidden filtered)" } else { "" };
    
    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(
                format!(" Files ({} items){} ", total_items, hidden_text),
                Style::default().fg(TEXT)
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)))
        .highlight_symbol(" ▸");

    f.render_stateful_widget(list, area, list_state);
}

/// Render a compact file list for all files view
pub fn render_compact_file_list(
    f: &mut Frame,
    area: Rect,
    entries: &[(usize, &FileEntry)],
    marked_for_deletion: &[usize],
    list_state: &mut ListState,
    title: &str,
) {
    let items: Vec<ListItem> = entries
        .iter()
        .map(|(actual_idx, entry)| {
            let category = Analyzer::categorize_file(entry);
            
            // Checkbox-style marker
            let marked = if marked_for_deletion.contains(actual_idx) {
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
            } else if marked_for_deletion.contains(actual_idx) {
                Style::default().fg(SUCCESS)
            } else {
                Style::default().fg(TEXT)
            };

            // Truncate path for display
            let path_display = entry.path.to_string_lossy();
            let display_text = if path_display.len() > 50 {
                format!("...{}", &path_display[path_display.len()-47..])
            } else {
                path_display.to_string()
            };
            
            let size_str = humansize::format_size(entry.size, humansize::DECIMAL);
            
            ListItem::new(vec![
                Line::from(vec![
                    marked,
                    icon,
                    Span::styled(entry.name.clone(), name_style.bold()),
                    Span::styled(format!("  {:>10}", size_str), Style::default().fg(ACCENT)),
                    Span::styled(format!("  {}", category.as_str()), Style::default().fg(category.color())),
                ]),
                Line::from(vec![
                    Span::styled("      ", Style::default()),
                    Span::styled(display_text, Style::default().fg(TEXT_DIM)),
                ]),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(title, Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)))
        .highlight_symbol(" ▸");

    f.render_stateful_widget(list, area, list_state);
}
