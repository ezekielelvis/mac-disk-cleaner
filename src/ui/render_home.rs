#![allow(dead_code)]

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
};
use crate::ui::types::{ScanOption, HomeMenuState};
use crate::ui::colors::*;

/// Render the home screen with scanning options
pub fn render_home(f: &mut Frame, menu_state: &HomeMenuState, frame_count: u32) {
    let area = f.area();
    f.render_widget(Clear, area);
    
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(8),   // Logo and title
            Constraint::Length(3),   // Storage overview bar
            Constraint::Min(16),     // Scan options
            Constraint::Length(4),   // Quick tips
            Constraint::Length(2),   // Footer
        ])
        .split(area);

    render_logo(f, main_chunks[0], frame_count);
    render_storage_overview(f, &menu_state.storage_info, main_chunks[1]);
    render_scan_options(f, menu_state, main_chunks[2]);
    render_quick_tips(f, main_chunks[3], frame_count);
    render_home_footer(f, main_chunks[4]);
}

fn render_logo(f: &mut Frame, area: Rect, frame_count: u32) {
    let logo = vec![
        "  ██████╗ ██╗███████╗██╗  ██╗",
        "  ██╔══██╗██║██╔════╝██║ ██╔╝",
        "  ██║  ██║██║███████╗█████╔╝ ",
        "  ██║  ██║██║╚════██║██╔═██╗ ",
        "  ██████╔╝██║███████║██║  ██╗",
        "  ╚═════╝ ╚═╝╚══════╝╚═╝  ╚═╝",
    ];

    // Gradient colors for animation
    let colors = [
        Color::Rgb(99, 102, 241),   // Indigo
        Color::Rgb(139, 92, 246),   // Purple
        Color::Rgb(168, 85, 247),   // Violet
        Color::Rgb(139, 92, 246),   // Purple
    ];
    let color_idx = ((frame_count / 10) % colors.len() as u32) as usize;
    let current_color = colors[color_idx];

    let logo_lines: Vec<Line> = logo.iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(current_color).bold())))
        .collect();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Length(2)])
        .split(area);

    let logo_widget = Paragraph::new(logo_lines)
        .alignment(Alignment::Center);
    f.render_widget(logo_widget, chunks[0]);

    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled("C L E A N E R", Style::default().fg(TEXT).bold()),
        Span::styled("  ·  ", Style::default().fg(MUTED)),
        Span::styled("Smart Disk Space Analyzer", Style::default().fg(TEXT_DIM)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(subtitle, chunks[1]);
}

fn render_storage_overview(f: &mut Frame, storage: &crate::ui::types::StorageInfo, area: Rect) {
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

fn render_scan_options(f: &mut Frame, menu_state: &HomeMenuState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Left: Scan options list
    let options = &menu_state.options;
    let items: Vec<ListItem> = options.iter().enumerate().map(|(i, opt)| {
        let is_selected = i == menu_state.selected_option;
        let (icon, title, desc, size_hint) = match opt {
            ScanOption::FullDisk => (
                "🌐",
                "Full Disk Scan",
                "Scan entire system for comprehensive analysis",
                "Scans: /"
            ),
            ScanOption::HomeDirectory => (
                "🏠",
                "Home Directory",
                "Scan your personal files and folders",
                "Scans: ~/"
            ),
            ScanOption::CustomPath => (
                "📁",
                "Custom Path",
                "Choose a specific directory to scan",
                if menu_state.custom_path.is_empty() { "Path: Not set" } else { &menu_state.custom_path }
            ),
            ScanOption::QuickScan => (
                "⚡",
                "Quick Scan",
                "Fast scan of common junk locations",
                "Cache, Logs, Temp files"
            ),
            ScanOption::LargeFiles => (
                "📦",
                "Large Files Only",
                "Find files larger than 100MB",
                "> 100 MB"
            ),
            ScanOption::OldFiles => (
                "📅",
                "Old & Unused Files",
                "Files not accessed in 6+ months",
                "> 6 months old"
            ),
        };

        let style = if is_selected {
            Style::default().bg(Color::Rgb(55, 55, 85))
        } else {
            Style::default()
        };

        let marker = if is_selected { "▸ " } else { "  " };
        
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(marker, Style::default().fg(ACCENT)),
                Span::styled(format!("{} ", icon), Style::default()),
                Span::styled(title, Style::default().fg(TEXT).bold()),
            ]),
            Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(desc, Style::default().fg(TEXT_DIM)),
            ]),
            Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(size_hint, Style::default().fg(MUTED).italic()),
            ]),
            Line::from(""),
        ]).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(" 🔍 Scan Options ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));

    f.render_widget(list, chunks[0]);

    // Right: Option details and settings
    render_option_details(f, menu_state, chunks[1]);
}

fn render_option_details(f: &mut Frame, menu_state: &HomeMenuState, area: Rect) {
    let selected = &menu_state.options[menu_state.selected_option];
    
    let (title, details) = match selected {
        ScanOption::FullDisk => (
            "Full Disk Scan",
            vec![
                ("Scope", "All mounted volumes"),
                ("Depth", "Unlimited"),
                ("Time", "~5-15 minutes"),
                ("Permissions", "Root may be needed"),
            ]
        ),
        ScanOption::HomeDirectory => (
            "Home Directory",
            vec![
                ("Scope", "User files only"),
                ("Depth", "Unlimited"),
                ("Time", "~1-5 minutes"),
                ("Safety", "No system files"),
            ]
        ),
        ScanOption::CustomPath => (
            "Custom Path",
            vec![
                ("Path", if menu_state.custom_path.is_empty() { "Press 'p' to set" } else { &menu_state.custom_path }),
                ("Depth", "Configurable"),
                ("Time", "Varies by size"),
                ("Tip", "Use Tab to autocomplete"),
            ]
        ),
        ScanOption::QuickScan => (
            "Quick Scan",
            vec![
                ("Targets", "Cache directories"),
                ("", "Log files"),
                ("", "Temp folders"),
                ("Time", "< 1 minute"),
            ]
        ),
        ScanOption::LargeFiles => (
            "Large Files",
            vec![
                ("Min Size", "> 100 MB"),
                ("Scope", "Home directory"),
                ("Sorted", "By size (largest first)"),
                ("Time", "~2-5 minutes"),
            ]
        ),
        ScanOption::OldFiles => (
            "Old Files",
            vec![
                ("Age", "> 6 months"),
                ("Criteria", "Last access time"),
                ("Scope", "Home directory"),
                ("Time", "~2-5 minutes"),
            ]
        ),
    };

    let mut lines = vec![
        Line::from(Span::styled(title, Style::default().fg(ACCENT).bold())),
        Line::from(""),
    ];

    for (label, value) in details {
        if label.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("           ", Style::default()),
                Span::styled(value, Style::default().fg(TEXT)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<10} ", label), Style::default().fg(MUTED)),
                Span::styled(value, Style::default().fg(TEXT)),
            ]));
        }
    }

    // Settings section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("─── Settings ───", Style::default().fg(Color::Rgb(55, 55, 75)))));
    lines.push(Line::from(vec![
        Span::styled("Min Size   ", Style::default().fg(MUTED)),
        Span::styled(format!("{} MB", menu_state.min_size_mb), Style::default().fg(TEXT)),
        Span::styled("  [+/-]", Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Max Depth  ", Style::default().fg(MUTED)),
        Span::styled(
            if menu_state.max_depth == 0 { "Unlimited".to_string() } else { format!("{} levels", menu_state.max_depth) },
            Style::default().fg(TEXT)
        ),
        Span::styled("  [d]", Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Hidden     ", Style::default().fg(MUTED)),
        Span::styled(
            if menu_state.include_hidden { "Include" } else { "Exclude" },
            Style::default().fg(TEXT)
        ),
        Span::styled("  [.]", Style::default().fg(TEXT_DIM)),
    ]));

    let details_block = Paragraph::new(lines)
        .block(Block::default()
            .title(Span::styled(" ⚙ Details ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .wrap(Wrap { trim: true });

    f.render_widget(details_block, area);
}

fn render_quick_tips(f: &mut Frame, area: Rect, frame_count: u32) {
    let tips = [
        "💡 Use 'Quick Scan' for fast cleanup of temporary files",
        "💡 'Large Files' mode helps find forgotten downloads",
        "💡 System files are protected and cannot be deleted",
        "💡 Press '?' during scan results for keyboard shortcuts",
        "💡 Use category view (v) to see files grouped by type",
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

fn render_home_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(ACCENT)),
        Span::styled(" Select  ", Style::default().fg(MUTED)),
        Span::styled("Enter", Style::default().fg(ACCENT)),
        Span::styled(" Start Scan  ", Style::default().fg(MUTED)),
        Span::styled("p", Style::default().fg(ACCENT)),
        Span::styled(" Set Path  ", Style::default().fg(MUTED)),
        Span::styled("+/-", Style::default().fg(ACCENT)),
        Span::styled(" Min Size  ", Style::default().fg(MUTED)),
        Span::styled("q", Style::default().fg(ACCENT)),
        Span::styled(" Quit", Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}

/// Render the path input modal
pub fn render_path_input(f: &mut Frame, input: &str, _cursor_pos: usize, suggestions: &[String]) {
    let area = f.area();
    
    // Center the modal
    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 12.min(area.height.saturating_sub(4));
    let modal_x = (area.width - modal_width) / 2;
    let modal_y = (area.height - modal_height) / 2;
    
    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);
    
    // Clear and draw modal background
    f.render_widget(Clear, modal_area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // Label
            Constraint::Length(3),  // Input
            Constraint::Min(3),     // Suggestions
            Constraint::Length(1),  // Help
        ])
        .split(modal_area);

    let modal_block = Block::default()
        .title(Span::styled(" 📁 Enter Path ", Style::default().fg(ACCENT)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Rgb(30, 30, 45)));
    f.render_widget(modal_block, modal_area);

    // Input field
    let display_input = if input.is_empty() { "/path/to/scan" } else { input };
    let input_style = if input.is_empty() { Style::default().fg(MUTED) } else { Style::default().fg(TEXT) };
    
    let input_widget = Paragraph::new(display_input)
        .style(input_style)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    f.render_widget(input_widget, chunks[1]);

    // Suggestions
    if !suggestions.is_empty() {
        let suggestion_items: Vec<ListItem> = suggestions.iter().take(5).map(|s| {
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(s, Style::default().fg(TEXT_DIM)),
            ]))
        }).collect();
        
        let suggestions_list = List::new(suggestion_items)
            .block(Block::default()
                .title(Span::styled(" Suggestions (Tab to complete) ", Style::default().fg(MUTED)))
                .borders(Borders::NONE));
        f.render_widget(suggestions_list, chunks[2]);
    }

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(ACCENT)),
        Span::styled(" Confirm  ", Style::default().fg(MUTED)),
        Span::styled("Tab", Style::default().fg(ACCENT)),
        Span::styled(" Complete  ", Style::default().fg(MUTED)),
        Span::styled("Esc", Style::default().fg(ACCENT)),
        Span::styled(" Cancel", Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);
}
