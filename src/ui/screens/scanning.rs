use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use crate::ui::app::App;
use crate::ui::colors::*;
use crate::ui::components::{render_scan_header, render_stats_panel, render_scan_footer, render_compact_storage_gauge};

/// Render the enhanced scanning view with animated indicators (no progress bar)
pub fn render_scanning_enhanced(f: &mut Frame, app: &App, frame_count: u32, _scroll_offset: usize) {
    let area = f.area();
    f.render_widget(Clear, area);
    
    // Responsive margin
    let margin = if area.width < 80 { 1 } else { 1 };
    
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Length(2),   // Storage bar
            Constraint::Length(6),   // Stats panel with cards
            Constraint::Min(10),     // Active path tree display
            Constraint::Length(2),   // Footer
        ])
        .split(area);

    render_scan_header(f, main_chunks[0], frame_count);
    render_compact_storage_gauge(f, &app.storage_info, main_chunks[1]);
    
    let snap = &app.last_progress_snapshot;
    render_stats_panel(f, main_chunks[2], snap.files_scanned, snap.dirs_scanned, snap.total_size_scanned, snap.entries_count, frame_count);
    
    render_active_scan_display(f, app, main_chunks[3], frame_count);
    render_scan_footer(f, main_chunks[4]);
}

fn render_active_scan_display(f: &mut Frame, app: &App, area: Rect, frame_count: u32) {
    let snap = &app.last_progress_snapshot;
    
    // Split into path tree and category breakdown
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    
    // Left: Active path tree
    render_path_tree(f, &snap.current_path, chunks[0], frame_count);
    
    // Right: Category pie chart breakdown
    render_category_breakdown(f, &snap.category_sizes, snap.total_size_scanned, chunks[1], frame_count);
}

fn render_path_tree(f: &mut Frame, current_path: &str, area: Rect, frame_count: u32) {
    let path_display = if current_path.is_empty() {
        "Initializing scan...".to_string()
    } else {
        current_path.to_string()
    };

    // Animated pulse indicator
    let pulse_chars = ["●", "◐", "○", "◑"];
    let pulse = pulse_chars[((frame_count / 8) % 4) as usize];

    // Split path into segments and build tree
    let segments: Vec<&str> = path_display.split('/').filter(|s| !s.is_empty()).collect();
    
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", pulse), Style::default().fg(ACCENT)),
            Span::styled("Active Scan Path", Style::default().fg(TEXT).bold()),
        ]),
        Line::from(""),
    ];
    
    if segments.is_empty() {
        lines.push(Line::from(Span::styled("  Initializing...", Style::default().fg(MUTED))));
    } else {
        // Show root
        lines.push(Line::from(vec![
            Span::styled("  /", Style::default().fg(TEXT_DIM)),
        ]));
        
        // Build tree structure - limit depth for display
        let max_display = 8.min(segments.len());
        let start_idx = if segments.len() > max_display { segments.len() - max_display } else { 0 };
        
        for (display_i, i) in (start_idx..segments.len()).enumerate() {
            let segment = segments[i];
            let is_last = i == segments.len() - 1;
            let depth = (display_i + 1).min(8);
            
            // Create indentation with tree lines
            let mut indent_spans = vec![Span::styled("  ", Style::default())];
            for d in 0..depth.saturating_sub(1) {
                if d < display_i {
                    indent_spans.push(Span::styled("│   ", Style::default().fg(Color::Rgb(55, 55, 75))));
                } else {
                    indent_spans.push(Span::styled("    ", Style::default()));
                }
            }
            
            // Tree branch character
            let branch = if is_last { "└── " } else { "├── " };
            indent_spans.push(Span::styled(branch, Style::default().fg(Color::Rgb(75, 75, 95))));
            
            // Segment name with styling
            let (icon, style) = if is_last {
                ("📂 ", Style::default().fg(ACCENT).bold())
            } else {
                ("📁 ", Style::default().fg(TEXT_DIM))
            };
            
            indent_spans.push(Span::styled(icon, style));
            
            // Truncate long segment names
            let display_name = if segment.len() > 20 {
                format!("{}...", &segment[..17])
            } else {
                segment.to_string()
            };
            indent_spans.push(Span::styled(display_name, style));
            
            // Add animated indicator for active path
            if is_last {
                let scan_dots = match (frame_count / 10) % 4 {
                    0 => " .",
                    1 => " ..",
                    2 => " ...",
                    _ => "",
                };
                indent_spans.push(Span::styled(scan_dots, Style::default().fg(ACCENT)));
            }
            
            lines.push(Line::from(indent_spans));
        }
        
        // Show ellipsis if path was truncated
        if start_idx > 0 {
            lines.insert(3, Line::from(vec![
                Span::styled("    ⋮ ", Style::default().fg(MUTED)),
                Span::styled(format!("({} more levels)", start_idx), Style::default().fg(TEXT_DIM)),
            ]));
        }
    }
    
    // Pad remaining lines
    while lines.len() < area.height as usize {
        lines.push(Line::from(""));
    }

    let path_widget = Paragraph::new(lines)
        .block(Block::default()
            .title(Span::styled(" 🔍 Scanning ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    
    f.render_widget(path_widget, area);
}

fn render_category_breakdown(f: &mut Frame, category_sizes: &std::collections::HashMap<String, u64>, total_size: u64, area: Rect, frame_count: u32) {
    // Define category colors for the pie chart segments
    let category_colors: Vec<(&str, Color)> = vec![
        ("🗑️ Cache", Color::Yellow),
        ("🌡️ Temp Files", Color::Red),
        ("📦 Large Files", Color::Magenta),
        ("📅 Old Files", Color::Cyan),
        ("👯 Duplicate Names", Color::Blue),
        ("📜 Log Files", Color::LightYellow),
        ("🔨 Build Artifacts", Color::LightRed),
        ("📦 node_modules", Color::LightMagenta),
        ("📥 Package Cache", Color::LightCyan),
        ("👁️ Hidden Files", Color::Gray),
        ("⚙️ System Files", Color::Red),
        ("📚 Library Files", Color::LightBlue),
        ("⬇️ Downloads", Color::Green),
        ("📄 Documents", Color::White),
        ("🎬 Media", Color::LightGreen),
        ("🗜️ Archives", Color::Yellow),
        ("📁 Regular", Color::White),
    ];
    
    // Sort categories by size (largest first)
    let mut sorted_categories: Vec<(&String, &u64)> = category_sizes.iter().collect();
    sorted_categories.sort_by(|a, b| b.1.cmp(a.1));
    
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" 📊 ", Style::default().fg(ACCENT)),
            Span::styled("Category Breakdown", Style::default().fg(TEXT).bold()),
        ]),
        Line::from(""),
    ];
    
    if sorted_categories.is_empty() || total_size == 0 {
        lines.push(Line::from(Span::styled("  Scanning...", Style::default().fg(MUTED))));
    } else {
        // Build ASCII pie chart representation
        let pie_width = 20;
        
        // Calculate percentages and build pie segments
        let mut pie_segments: Vec<(String, f64, Color)> = Vec::new();
        for (cat_name, size) in sorted_categories.iter().take(8) {
            let size_val = **size;
            let percentage = if total_size > 0 { size_val as f64 / total_size as f64 * 100.0 } else { 0.0 };
            if percentage >= 1.0 {  // Only show categories with >= 1%
                let color = category_colors.iter()
                    .find(|(name, _)| cat_name.contains(name.split(' ').last().unwrap_or("")))
                    .map(|(_, c)| *c)
                    .unwrap_or(TEXT_DIM);
                pie_segments.push((cat_name.to_string(), percentage, color));
            }
        }
        
        // Draw a horizontal bar chart (visual pie representation)
        let bar_total = pie_width as f64;
        let mut bar_line_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
        
        for (_, percentage, color) in &pie_segments {
            let segment_width = ((percentage / 100.0) * bar_total).max(1.0) as usize;
            bar_line_spans.push(Span::styled(
                "█".repeat(segment_width),
                Style::default().fg(*color)
            ));
        }
        
        lines.push(Line::from(bar_line_spans));
        lines.push(Line::from(""));
        
        // Animated indicator
        let pulse_chars = ["●", "◐", "○", "◑"];
        let pulse = pulse_chars[((frame_count / 8) % 4) as usize];
        
        // List categories with sizes
        let max_items = (area.height.saturating_sub(6)) as usize;
        
        for (i, (cat_name, size)) in sorted_categories.iter().take(max_items).enumerate() {
            let size_val = **size;
            let percentage = if total_size > 0 { size_val as f64 / total_size as f64 * 100.0 } else { 0.0 };
            
            let color = category_colors.iter()
                .find(|(name, _)| cat_name.contains(name.split(' ').last().unwrap_or("")))
                .map(|(_, c)| *c)
                .unwrap_or(TEXT_DIM);
            
            let size_str = humansize::format_size(size_val, humansize::DECIMAL);
            
            // Truncate category name if needed
            let display_name = if cat_name.len() > 16 {
                format!("{}...", &cat_name[..13])
            } else {
                format!("{:<16}", cat_name)
            };
            
            // Animated indicator for top category
            let indicator = if i == 0 { 
                Span::styled(format!(" {} ", pulse), Style::default().fg(ACCENT))
            } else {
                Span::styled("   ", Style::default())
            };
            
            lines.push(Line::from(vec![
                indicator,
                Span::styled("█ ", Style::default().fg(color)),
                Span::styled(display_name, Style::default().fg(TEXT)),
                Span::styled(format!("{:>8}", size_str), Style::default().fg(TEXT_DIM)),
                Span::styled(format!(" {:>5.1}%", percentage), Style::default().fg(MUTED)),
            ]));
        }
        
        // Show "other" if there are more categories
        if sorted_categories.len() > max_items {
            let other_count = sorted_categories.len() - max_items;
            let other_size: u64 = sorted_categories.iter().skip(max_items).map(|(_, s)| **s).sum();
            let other_percentage = if total_size > 0 { other_size as f64 / total_size as f64 * 100.0 } else { 0.0 };
            
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled("░ ", Style::default().fg(MUTED)),
                Span::styled(format!("+{} more", other_count), Style::default().fg(TEXT_DIM)),
                Span::styled(format!("{:>8}", humansize::format_size(other_size, humansize::DECIMAL)), Style::default().fg(TEXT_DIM)),
                Span::styled(format!(" {:>5.1}%", other_percentage), Style::default().fg(MUTED)),
            ]));
        }
    }

    let category_widget = Paragraph::new(lines)
        .block(Block::default()
            .title(Span::styled(" 📈 Categories Found ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    
    f.render_widget(category_widget, area);
}
