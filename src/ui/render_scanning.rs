use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
};
use crate::ui::app::App;
use crate::ui::colors::*;

/// Render the enhanced scanning view with charts and scrolling file list
pub fn render_scanning_enhanced(f: &mut Frame, app: &App, frame_count: u32, _scroll_offset: usize) {
    let area = f.area();
    f.render_widget(Clear, area);
    
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Length(5),   // Storage and progress
            Constraint::Length(6),   // Stats panel with mini charts
            Constraint::Min(12),     // Active path tree display
            Constraint::Length(2),   // Footer
        ])
        .split(area);

    render_scan_header(f, main_chunks[0], frame_count);
    render_progress_section(f, app, main_chunks[1], frame_count);
    render_stats_panel(f, app, main_chunks[2], frame_count);
    render_active_path_tree(f, app, main_chunks[3], frame_count);
    render_scan_footer(f, main_chunks[4]);
}

fn render_scan_header(f: &mut Frame, area: Rect, frame_count: u32) {
    // Animated scanning indicator
    let dots = match (frame_count / 15) % 4 {
        0 => "   ",
        1 => ".  ",
        2 => ".. ",
        _ => "...",
    };
    
    let spinner = match (frame_count / 5) % 4 {
        0 => "◐",
        1 => "◓",
        2 => "◑",
        _ => "◒",
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(format!("{} ", spinner), Style::default().fg(ACCENT)),
        Span::styled("SCANNING", Style::default().fg(TEXT).bold()),
        Span::styled(dots, Style::default().fg(ACCENT)),
        Span::styled("  Analyzing your disk", Style::default().fg(TEXT_DIM)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    
    f.render_widget(header, area);
}

fn render_progress_section(f: &mut Frame, app: &App, area: Rect, frame_count: u32) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Storage bar
            Constraint::Length(3),  // Progress bar
        ])
        .split(area);

    // Storage bar
    let storage = &app.storage_info;
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { ACCENT };
    
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    
    let storage_gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(35, 35, 50)))
        .ratio(usage.min(1.0))
        .label(Span::styled(
            format!("💾 Storage: {} / {} ({:.0}% used)", used, total, usage * 100.0),
            Style::default().fg(TEXT)
        ));
    f.render_widget(storage_gauge, chunks[0]);

    // Animated scan progress bar
    let snap = &app.last_progress_snapshot;
    let scanned_size = humansize::format_size(snap.total_size_scanned, humansize::DECIMAL);
    
    // Create an animated sweep effect
    let progress = (frame_count % 100) as f64 / 100.0;
    
    let progress_gauge = Gauge::default()
        .gauge_style(Style::default().fg(ACCENT).bg(Color::Rgb(35, 35, 50)))
        .ratio(progress)
        .label(Span::styled(
            format!("📊 Scanned: {} files, {} dirs, {}", 
                snap.files_scanned, snap.dirs_scanned, scanned_size),
            Style::default().fg(TEXT)
        ))
        .block(Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    
    f.render_widget(progress_gauge, chunks[1]);
}

fn render_stats_panel(f: &mut Frame, app: &App, area: Rect, frame_count: u32) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),  // Files stat
            Constraint::Percentage(25),  // Dirs stat
            Constraint::Percentage(25),  // Size stat
            Constraint::Percentage(25),  // Rate stat
        ])
        .split(area);

    let snap = &app.last_progress_snapshot;

    // Files scanned card
    render_stat_card(f, chunks[0], "📄 FILES", &format!("{}", snap.files_scanned), ACCENT, frame_count);
    
    // Directories scanned card
    render_stat_card(f, chunks[1], "📁 DIRS", &format!("{}", snap.dirs_scanned), SUCCESS, frame_count);
    
    // Total size card
    let size_str = humansize::format_size(snap.total_size_scanned, humansize::DECIMAL);
    render_stat_card(f, chunks[2], "💾 SIZE", &size_str, WARNING, frame_count);
    
    // Items found card
    render_stat_card(f, chunks[3], "🔍 FOUND", &format!("{}", snap.entries_count), Color::Rgb(168, 85, 247), frame_count);
}

fn render_stat_card(f: &mut Frame, area: Rect, label: &str, value: &str, color: Color, _frame_count: u32) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // Label
            Constraint::Length(2),  // Value
            Constraint::Min(1),     // Mini sparkline placeholder
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

    // Mini activity indicator
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60)));
    f.render_widget(block, area);
}

fn render_active_path_tree(f: &mut Frame, app: &App, area: Rect, frame_count: u32) {
    let snap = &app.last_progress_snapshot;
    
    // Display the active scanning path in a tree format
    let path_display = if snap.current_path.is_empty() {
        "Initializing scan...".to_string()
    } else {
        snap.current_path.clone()
    };

    // Animated scanning indicator
    let pulse = match (frame_count / 8) % 4 {
        0 => "●",
        1 => "◐",
        2 => "○",
        _ => "◑",
    };

    // Split path into segments and build tree
    let segments: Vec<&str> = path_display.split('/').filter(|s| !s.is_empty()).collect();
    
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", pulse), Style::default().fg(ACCENT)),
            Span::styled("Scanning Path Tree", Style::default().fg(TEXT).bold()),
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
        
        // Build tree structure
        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;
            let depth = (i + 1).min(8);
            
            // Create indentation with tree lines
            let mut indent_spans = vec![Span::styled("  ", Style::default())];
            for d in 0..depth.saturating_sub(1) {
                if d < segments.len() - 1 {
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
            indent_spans.push(Span::styled(*segment, style));
            
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
    }
    
    // Pad remaining lines
    while lines.len() < area.height as usize {
        lines.push(Line::from(""));
    }

    let path_widget = Paragraph::new(lines)
        .block(Block::default()
            .title(Span::styled(
                " 🔍 Active Scan Location ",
                Style::default().fg(TEXT)
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    
    f.render_widget(path_widget, area);
}

fn render_scan_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Q", Style::default().fg(DANGER)),
        Span::styled(" Cancel scan  ", Style::default().fg(MUTED)),
        Span::styled("•", Style::default().fg(Color::Rgb(45, 45, 60))),
        Span::styled("  Scan will complete automatically", Style::default().fg(TEXT_DIM)),
    ]))
    .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}
