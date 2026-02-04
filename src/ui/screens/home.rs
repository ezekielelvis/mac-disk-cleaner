use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};
use crate::ui::types::{ScanOption, HomeMenuState};
use crate::ui::colors::*;
use crate::ui::components::{render_logo, render_storage_overview, render_quick_tips, render_home_footer};

/// Render the home screen with scanning options - responsive layout
pub fn render_home(f: &mut Frame, menu_state: &HomeMenuState, frame_count: u32) {
    let area = f.area();
    f.render_widget(Clear, area);
    
    // Responsive margin based on screen size
    let margin = if area.width < 80 { 1 } else { 2 };
    
    // Calculate responsive constraints
    let logo_height = if area.height < 30 { 6 } else { 8 };
    let storage_height = 3;
    let tips_height = if area.height < 25 { 2 } else { 4 };
    let footer_height = 2;
    
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints([
            Constraint::Length(logo_height),    // Logo and title
            Constraint::Length(storage_height), // Storage overview bar
            Constraint::Min(12),                // Scan options (takes remaining space)
            Constraint::Length(tips_height),    // Quick tips
            Constraint::Length(footer_height),  // Footer
        ])
        .split(area);

    render_logo(f, main_chunks[0], frame_count);
    render_storage_overview(f, &menu_state.storage_info, main_chunks[1]);
    render_scan_options(f, menu_state, main_chunks[2]);
    render_quick_tips(f, main_chunks[3], frame_count);
    render_home_footer(f, main_chunks[4]);
}

fn render_scan_options(f: &mut Frame, menu_state: &HomeMenuState, area: Rect) {
    // Responsive layout - stack vertically on narrow screens
    let is_narrow = area.width < 90;
    
    if is_narrow {
        // Single column layout
        render_scan_options_list(f, menu_state, area);
    } else {
        // Two column layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        render_scan_options_list(f, menu_state, chunks[0]);
        render_option_details(f, menu_state, chunks[1]);
    }
}

fn render_scan_options_list(f: &mut Frame, menu_state: &HomeMenuState, area: Rect) {
    let options = &menu_state.options;
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    let items_per_option = 4; // Lines per option item
    let total_items = options.len();
    
    // Calculate scroll position to keep selected item visible
    let selected = menu_state.selected_option;
    let _scroll_offset = if selected * items_per_option >= visible_height {
        (selected * items_per_option).saturating_sub(visible_height / 2)
    } else {
        0
    };
    
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
                if menu_state.custom_path.is_empty() { "Path: Press 'p' to set" } else { &menu_state.custom_path }
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
            .title(Span::styled(" 🔍 Scan Options (↑↓ to select, Enter to start) ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));

    f.render_widget(list, area);
    
    // Render scrollbar if needed
    if total_items * items_per_option > visible_height {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(Color::Rgb(75, 75, 95)));
        
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(total_items)
            .position(selected);
        
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
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
