use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
};
use crate::analyzer::Analyzer;
use crate::ui::app::App;
use crate::ui::types::{AppState, ViewMode};
use crate::ui::colors::*;
use crate::ui::components::render_stat_block;

/// Render the scan complete summary screen with quick actions
pub fn render_scan_complete(f: &mut Frame, app: &App, area: Rect) {
    // Responsive constraints
    let margin = if area.width < 80 { 1 } else { 2 };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints([
            Constraint::Length(5),    // Header with checkmark
            Constraint::Length(8),    // Summary stats
            Constraint::Length(10),   // Top categories
            Constraint::Min(4),       // Recommendations  
            Constraint::Length(4),    // Quick actions
        ])
        .split(area);

    // Header - Success message
    let header = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(SUCCESS).bold()),
            Span::styled("Scan Complete!", Style::default().fg(TEXT).bold()),
        ]),
        Line::from(""),
    ])
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    f.render_widget(header, chunks[0]);

    // Summary stats
    if let Some(ref result) = app.scan_result {
        let safe_size = Analyzer::calculate_safe_savings(&result.entries);
        
        let stats_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[1]);

        let files_block = render_stat_block("📄 FILES", &format!("{}", result.total_files), ACCENT);
        f.render_widget(files_block, stats_chunks[0]);

        let dirs_block = render_stat_block("📁 DIRECTORIES", &format!("{}", result.total_dirs), SUCCESS);
        f.render_widget(dirs_block, stats_chunks[1]);

        let total_block = render_stat_block("💾 DISK USED", &humansize::format_size(app.storage_info.used_space, humansize::DECIMAL), WARNING);
        f.render_widget(total_block, stats_chunks[2]);

        let savings_block = render_stat_block("🎯 SAFE TO DELETE", &humansize::format_size(safe_size, humansize::DECIMAL), Color::Rgb(34, 197, 94));
        f.render_widget(savings_block, stats_chunks[3]);

        // Top categories
        let mut categories: Vec<_> = app.categories.iter().collect();
        categories.sort_by(|a, b| {
            let size_a: u64 = a.1.iter().map(|e| e.size).sum();
            let size_b: u64 = b.1.iter().map(|e| e.size).sum();
            size_b.cmp(&size_a)
        });

        let cat_lines: Vec<Line> = categories.iter()
            .take(5)
            .map(|(category, entries)| {
                let total_size: u64 = entries.iter().map(|e| e.size).sum();
                let cat_name = category.as_str().to_string();
                let is_safe = category.is_safe_to_delete();
                
                Line::from(vec![
                    Span::styled(if is_safe { "  ✓ " } else { "  ! " }, 
                        Style::default().fg(if is_safe { SUCCESS } else { WARNING })),
                    Span::styled(format!("{:<20}", cat_name), Style::default().fg(category.color())),
                    Span::styled(format!("{:>6} items", entries.len()), Style::default().fg(TEXT_DIM)),
                    Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
                    Span::styled(format!("{:>10}", humansize::format_size(total_size, humansize::DECIMAL)), 
                        Style::default().fg(TEXT)),
                ])
            })
            .collect();

        let categories_widget = Paragraph::new(cat_lines)
            .block(Block::default()
                .title(Span::styled(" 📊 Top Categories ", Style::default().fg(TEXT)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
        f.render_widget(categories_widget, chunks[2]);

        // Recommendations
        let rec_lines: Vec<Line> = app.recommendations.iter()
            .take(3)
            .map(|r| Line::from(vec![
                Span::styled("  💡 ", Style::default()),
                Span::styled(r, Style::default().fg(TEXT_DIM)),
            ]))
            .collect();

        let recs_widget = Paragraph::new(rec_lines)
            .block(Block::default()
                .title(Span::styled(" 💡 Recommendations ", Style::default().fg(TEXT)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
        f.render_widget(recs_widget, chunks[3]);
    }

    // Quick actions - emphasize browse and delete workflow
    let actions = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Enter/b", Style::default().fg(SUCCESS).bold()),
            Span::styled(" 📂 Browse & Delete    ", Style::default().fg(TEXT)),
            Span::styled("f", Style::default().fg(ACCENT).bold()),
            Span::styled(" All Files    ", Style::default().fg(MUTED)),
            Span::styled("d", Style::default().fg(WARNING).bold()),
            Span::styled(" Details    ", Style::default().fg(MUTED)),
            Span::styled("s", Style::default().fg(Color::Rgb(34, 197, 94)).bold()),
            Span::styled(" Select Safe    ", Style::default().fg(MUTED)),
            Span::styled("h", Style::default().fg(ACCENT).bold()),
            Span::styled(" Home    ", Style::default().fg(MUTED)),
            Span::styled("q", Style::default().fg(DANGER).bold()),
            Span::styled(" Quit", Style::default().fg(MUTED)),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    f.render_widget(actions, chunks[4]);
}

/// Render detailed scan results
pub fn render_scan_details(f: &mut Frame, app: &App, area: Rect) {
    let margin = if area.width < 80 { 1 } else { 1 };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints([
            Constraint::Length(4),    // Header
            Constraint::Length(6),    // Disk usage
            Constraint::Length(8),    // Category breakdown
            Constraint::Min(6),       // Analysis
            Constraint::Length(3),    // Actions
        ])
        .split(area);

    render_scan_details_header(f, app, chunks[0]);
    render_disk_usage_breakdown(f, app, chunks[1]);
    render_category_breakdown_detailed(f, app, chunks[2]);
    render_detailed_analysis(f, app, chunks[3]);
    render_scan_details_actions(f, app, chunks[4]);
}

fn render_scan_details_header(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref result) = app.scan_result {
        let safe_size = Analyzer::calculate_safe_savings(&result.entries);
        let total_entries = result.entries.len();
        let scan_path = app.scan_path.display().to_string();
        
        let header_lines = vec![
            Line::from(vec![
                Span::styled("🔍 ", Style::default().fg(ACCENT)),
                Span::styled("Detailed Scan Results", Style::default().fg(TEXT).bold()),
                Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
                Span::styled(format!("Path: {}", if scan_path.len() > 40 { 
                    format!("...{}", &scan_path[scan_path.len()-37..])
                } else { 
                    scan_path 
                }), Style::default().fg(TEXT_DIM)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  📊 ", Style::default()),
                Span::styled(format!("{} total items", total_entries), Style::default().fg(ACCENT)),
                Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
                Span::styled(format!("{} files", result.total_files), Style::default().fg(TEXT)),
                Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
                Span::styled(format!("{} directories", result.total_dirs), Style::default().fg(TEXT)),
                Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
                Span::styled(format!("{} potential savings", humansize::format_size(safe_size, humansize::DECIMAL)), Style::default().fg(SUCCESS)),
            ]),
        ];

        let header = Paragraph::new(header_lines)
            .block(Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
        f.render_widget(header, area);
    }
}

fn render_disk_usage_breakdown(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let storage = &app.storage_info;
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { SUCCESS };
    
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    let free = humansize::format_size(storage.available_space, humansize::DECIMAL);
    
    let storage_widget = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("💾 Disk Usage:  ", Style::default().fg(MUTED)),
            Span::styled(format!("{:.1}%", usage * 100.0), Style::default().fg(bar_color).bold()),
        ]),
        Line::from(vec![
            Span::styled("   Used:       ", Style::default().fg(MUTED)),
            Span::styled(used, Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("   Total:      ", Style::default().fg(MUTED)),
            Span::styled(total, Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("   Available:  ", Style::default().fg(MUTED)),
            Span::styled(free, Style::default().fg(SUCCESS)),
        ]),
    ])
    .block(Block::default()
        .title(" 📊 Storage Analysis ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    f.render_widget(storage_widget, chunks[0]);

    if let Some(ref result) = app.scan_result {
        let regular_files = result.entries.iter().filter(|e| !e.is_dir && !e.is_hidden && !e.is_system).count();
        let hidden_files = result.hidden_count;
        let system_files = result.system_count;
        let total_files = result.total_files.max(1);

        let type_widget = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("📄 File Types", Style::default().fg(TEXT).bold()),
            ]),
            Line::from(vec![
                Span::styled("   Regular:    ", Style::default().fg(MUTED)),
                Span::styled(format!("{:>6} ({:.1}%)", regular_files, (regular_files as f64 / total_files as f64) * 100.0), Style::default().fg(ACCENT)),
            ]),
            Line::from(vec![
                Span::styled("   Hidden:     ", Style::default().fg(MUTED)),
                Span::styled(format!("{:>6} ({:.1}%)", hidden_files, (hidden_files as f64 / total_files as f64) * 100.0), Style::default().fg(WARNING)),
            ]),
            Line::from(vec![
                Span::styled("   System:     ", Style::default().fg(MUTED)),
                Span::styled(format!("{:>6} ({:.1}%)", system_files, (system_files as f64 / total_files as f64) * 100.0), Style::default().fg(DANGER)),
            ]),
        ])
        .block(Block::default()
            .title(" 🏷 File Distribution ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
        f.render_widget(type_widget, chunks[1]);
    }
}

fn render_category_breakdown_detailed(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let mut categories: Vec<_> = app.categories.iter().collect();
    categories.sort_by(|a, b| {
        let size_a: u64 = a.1.iter().map(|e| e.size).sum();
        let size_b: u64 = b.1.iter().map(|e| e.size).sum();
        size_b.cmp(&size_a)
    });

    let grand_total: u64 = categories.iter()
        .map(|(_, entries)| entries.iter().map(|e| e.size).sum::<u64>())
        .sum();

    let mut cat_lines = vec![Line::from("")];
    
    for (i, (category, entries)) in categories.iter().take(5).enumerate() {
        let total_size: u64 = entries.iter().map(|e| e.size).sum();
        let percentage = if grand_total > 0 {
            total_size as f64 / grand_total as f64 * 100.0
        } else {
            0.0
        };
        
        let is_safe = category.is_safe_to_delete();
        let safety_icon = if is_safe { "✓" } else { "!" };
        let safety_color = if is_safe { SUCCESS } else { WARNING };
        
        let bar_width = ((percentage / 100.0) * 15.0) as usize;
        let bar = "█".repeat(bar_width) + &"░".repeat(15_usize.saturating_sub(bar_width));
        
        cat_lines.push(Line::from(vec![
            Span::styled(format!(" {} ", safety_icon), Style::default().fg(safety_color)),
            Span::styled(format!("{:<14}", category.as_str()), Style::default().fg(category.color())),
            Span::styled(bar, Style::default().fg(category.color())),
            Span::styled(format!(" {:>5.1}%", percentage), Style::default().fg(TEXT_DIM)),
        ]));
        
        if i < 4 { cat_lines.push(Line::from("")); }
    }

    let categories_widget = Paragraph::new(cat_lines)
        .block(Block::default()
            .title(" 📂 Category Breakdown ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    f.render_widget(categories_widget, chunks[0]);

    // Safety summary
    let safe_categories: Vec<_> = categories.iter().filter(|(cat, _)| cat.is_safe_to_delete()).collect();
    let safe_total: u64 = safe_categories.iter()
        .map(|(_, entries)| entries.iter().map(|e| e.size).sum::<u64>())
        .sum();
    
    let safety_widget = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled("🛡 Safety Analysis", Style::default().fg(TEXT).bold())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✓ Safe to delete:", Style::default().fg(SUCCESS)),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(humansize::format_size(safe_total, humansize::DECIMAL), Style::default().fg(SUCCESS)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(MUTED)),
            Span::styled("s", Style::default().fg(SUCCESS)),
            Span::styled(" to select all safe items", Style::default().fg(MUTED)),
        ]),
    ])
    .block(Block::default()
        .title(" ⚖ Safety ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    f.render_widget(safety_widget, chunks[1]);
}

fn render_detailed_analysis(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref result) = app.scan_result {
        let mut large_files: Vec<_> = result.entries.iter()
            .filter(|e| !e.is_dir && e.size > 10 * 1024 * 1024)
            .collect();
        large_files.sort_by(|a, b| b.size.cmp(&a.size));
        
        let mut large_lines = vec![Line::from("")];
        for (i, entry) in large_files.iter().take(5).enumerate() {
            let name_display = if entry.name.len() > 25 {
                format!("{}...", &entry.name[..22])
            } else {
                entry.name.clone()
            };
            
            let category = Analyzer::categorize_file(entry);
            
            large_lines.push(Line::from(vec![
                Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(MUTED)),
                Span::styled(name_display, Style::default().fg(TEXT)),
                Span::styled(format!("  {:>10}", humansize::format_size(entry.size, humansize::DECIMAL)), Style::default().fg(ACCENT)),
                Span::styled(format!("  {}", category.as_str()), Style::default().fg(category.color())),
            ]));
        }

        let large_widget = Paragraph::new(large_lines)
            .block(Block::default()
                .title(format!(" 📊 Largest Files ({} files >10MB) ", large_files.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
        f.render_widget(large_widget, area);
    }
}

fn render_scan_details_actions(f: &mut Frame, _app: &App, area: Rect) {
    let actions = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(ACCENT).bold()),
            Span::styled(" Browse  ", Style::default().fg(MUTED)),
            Span::styled("f", Style::default().fg(SUCCESS).bold()),
            Span::styled(" All Files  ", Style::default().fg(MUTED)),
            Span::styled("c", Style::default().fg(SUCCESS).bold()),
            Span::styled(" Categories  ", Style::default().fg(MUTED)),
            Span::styled("s", Style::default().fg(SUCCESS).bold()),
            Span::styled(" Select Safe  ", Style::default().fg(MUTED)),
            Span::styled("Esc", Style::default().fg(WARNING).bold()),
            Span::styled(" Back  ", Style::default().fg(MUTED)),
            Span::styled("q", Style::default().fg(DANGER).bold()),
            Span::styled(" Quit", Style::default().fg(MUTED)),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    f.render_widget(actions, area);
}

/// Render the results browsing view
pub fn render_results_view(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),   // Header with storage
            Constraint::Length(3),   // Tab bar and breadcrumb
            Constraint::Min(10),     // Main content area
            Constraint::Length(3),   // Action bar
        ])
        .split(area);

    render_results_header(f, app, chunks[0]);
    render_navigation_bar(f, app, chunks[1]);
    render_main_content(f, app, chunks[2]);
    render_action_bar(f, app, chunks[3]);
}

fn render_results_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let stats = if let Some(ref result) = app.scan_result {
        let marked_size: u64 = app.marked_for_deletion.iter()
            .filter_map(|&i| result.entries.get(i))
            .map(|e| e.size)
            .sum();
        
        format!(
            "📄 {} files  📁 {} dirs  💾 {}  │  ✓ {} selected ({})",
            result.total_files,
            result.total_dirs,
            humansize::format_size(result.total_size, humansize::DECIMAL),
            app.marked_for_deletion.len(),
            humansize::format_size(marked_size, humansize::DECIMAL)
        )
    } else {
        String::new()
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("◉ ", Style::default().fg(ACCENT)),
        Span::styled("DISK CLEANER", Style::default().fg(TEXT).bold()),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
        Span::styled(stats, Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(header, chunks[0]);

    // Storage gauge
    let storage = &app.storage_info;
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
    f.render_widget(gauge, chunks[1]);
}

fn render_navigation_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(10)])
        .split(area);

    let tab_titles = vec![" 📁 Files ", " 📊 Categories "];
    let selected_tab = match app.current_view {
        ViewMode::AllFiles => 0,
        ViewMode::Categories => 1,
    };
    
    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(TEXT_DIM))
        .highlight_style(Style::default().fg(ACCENT).bold())
        .divider(Span::styled(" │ ", Style::default().fg(Color::Rgb(55, 55, 75))));
    
    f.render_widget(tabs, chunks[0]);

    let path_display = app.current_path.to_string_lossy();
    let truncated = if path_display.len() > 50 {
        format!("...{}", &path_display[path_display.len()-47..])
    } else {
        path_display.to_string()
    };

    let nav_indicator = if !app.navigation_stack.is_empty() {
        format!(" (←{} back)", app.navigation_stack.len())
    } else {
        String::new()
    };

    let breadcrumb = Paragraph::new(Line::from(vec![
        Span::styled("📂 ", Style::default()),
        Span::styled(truncated, Style::default().fg(TEXT)),
        Span::styled(nav_indicator, Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Right);
    
    f.render_widget(breadcrumb, chunks[1]);
}

fn render_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    if app.state == AppState::CategoryView {
        render_category_detail(f, app, chunks[0]);
    } else {
        match app.current_view {
            ViewMode::AllFiles => render_enhanced_file_list(f, app, chunks[0]),
            ViewMode::Categories => render_enhanced_categories(f, app, chunks[0]),
        }
    }

    render_details_panel(f, app, chunks[1]);
}

fn render_enhanced_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let current_entries = app.get_current_entries();
    let total_items = current_entries.len();
    
    let items: Vec<ListItem> = current_entries
        .iter()
        .map(|(actual_idx, entry)| {
            let category = Analyzer::categorize_file(entry);
            
            let marked = if app.marked_for_deletion.contains(actual_idx) {
                Span::styled("● ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("○ ", Style::default().fg(Color::Rgb(55, 55, 75)))
            };
            
            let icon = if entry.is_system {
                Span::styled("⚙ ", Style::default().fg(DANGER))
            } else if entry.is_hidden {
                Span::styled("◌ ", Style::default().fg(MUTED))
            } else if entry.is_dir {
                Span::styled("▶ ", Style::default().fg(ACCENT))
            } else {
                Span::styled("  ", Style::default())
            };
            
            let name_style = if entry.is_system {
                Style::default().fg(DANGER).dim()
            } else if entry.is_dir {
                Style::default().fg(TEXT).bold()
            } else if app.marked_for_deletion.contains(actual_idx) {
                Style::default().fg(SUCCESS)
            } else {
                Style::default().fg(TEXT)
            };

            let name_display = if entry.name.len() > 28 {
                format!("{}...", &entry.name[..25])
            } else {
                format!("{:<28}", entry.name)
            };
            
            // Use get_entry_display_size for folders to show calculated size
            let display_size = app.get_entry_display_size(entry);
            let size_str = humansize::format_size(display_size, humansize::DECIMAL);
            
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

    let hidden_text = if !app.show_hidden { " (hidden filtered)" } else { "" };
    let search_text = if !app.browse_search_query.is_empty() {
        format!(" [🔍\"{}\"]", app.browse_search_query)
    } else {
        String::new()
    };
    let sort_text = format!(" · {}", app.browse_sort_mode.name());
    
    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(
                format!(" Files ({} items){}{}{} ", total_items, hidden_text, search_text, sort_text),
                Style::default().fg(TEXT)
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)))
        .highlight_symbol(" ▸");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_enhanced_categories(f: &mut Frame, app: &mut App, area: Rect) {
    let mut categories: Vec<_> = app.categories.iter().collect();
    categories.sort_by(|a, b| {
        let size_a: u64 = a.1.iter().map(|e| e.size).sum();
        let size_b: u64 = b.1.iter().map(|e| e.size).sum();
        size_b.cmp(&size_a)
    });

    let grand_total: u64 = categories.iter()
        .map(|(_, entries)| entries.iter().map(|e| e.size).sum::<u64>())
        .sum();

    let items: Vec<ListItem> = categories
        .iter()
        .map(|(category, entries)| {
            let total_size: u64 = entries.iter().map(|e| e.size).sum();
            let percentage = if grand_total > 0 {
                (total_size as f64 / grand_total as f64 * 100.0) as u8
            } else {
                0
            };
            
            let safe_indicator = if category.is_safe_to_delete() {
                Span::styled("✓ ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("! ", Style::default().fg(WARNING))
            };
            
            let bar_width = (percentage as usize / 5).min(10);
            let bar = "█".repeat(bar_width) + &"░".repeat(10 - bar_width);
            
            ListItem::new(vec![
                Line::from(vec![
                    safe_indicator,
                    Span::styled(format!("{:<18}", category.as_str()), Style::default().fg(category.color()).bold()),
                    Span::styled(format!("{:>6} items", entries.len()), Style::default().fg(TEXT_DIM)),
                ]),
                Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(bar, Style::default().fg(category.color())),
                    Span::styled(format!(" {:>10} ", humansize::format_size(total_size, humansize::DECIMAL)), Style::default().fg(TEXT)),
                    Span::styled(format!("{}%", percentage), Style::default().fg(MUTED)),
                ]),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(
                format!(" Categories ({}) ", categories.len()),
                Style::default().fg(TEXT)
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)))
        .highlight_symbol(" ▸");

    f.render_stateful_widget(list, area, &mut app.category_state);
}

fn render_category_detail(f: &mut Frame, app: &App, area: Rect) {
    if let Some(category) = app.selected_category {
        if let Some(entries) = app.categories.get(&category) {
            let items: Vec<ListItem> = entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let icon = if entry.is_system {
                        Span::styled("⚙ ", Style::default().fg(DANGER))
                    } else if entry.is_dir {
                        Span::styled("▶ ", Style::default().fg(ACCENT))
                    } else {
                        Span::styled("  ", Style::default())
                    };

                    let name_display = if entry.name.len() > 35 {
                        format!("{}...", &entry.name[..32])
                    } else {
                        format!("{:<35}", entry.name)
                    };
                    
                    let size_str = humansize::format_size(entry.size, humansize::DECIMAL);
                    
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{:>3} ", i + 1), Style::default().fg(MUTED)),
                        icon,
                        Span::styled(name_display, Style::default().fg(TEXT)),
                        Span::styled(format!("{:>10}", size_str), Style::default().fg(ACCENT)),
                    ]))
                })
                .collect();

            let total_size: u64 = entries.iter().map(|e| e.size).sum();
            let safe_text = if category.is_safe_to_delete() { "✓ Safe" } else { "! Review" };
            
            let list = List::new(items)
                .block(Block::default()
                    .title(Span::styled(
                        format!(" {} · {} · {} items · {} ", 
                            category.as_str(), 
                            safe_text,
                            entries.len(),
                            humansize::format_size(total_size, humansize::DECIMAL)
                        ),
                        Style::default().fg(category.color())
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(category.color())))
                .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)));

            f.render_widget(list, area);
            return;
        }
    }
    
    let empty = Paragraph::new("No category selected")
        .style(Style::default().fg(MUTED))
        .block(Block::default()
            .title(" Category Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    f.render_widget(empty, area);
}

fn render_details_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Details section
    let details = if let Some(visible_idx) = app.list_state.selected() {
        let current_entries = app.get_current_entries();
        if let Some((_, entry)) = current_entries.get(visible_idx) {
            let category = Analyzer::categorize_file(entry);
            let cat_name = category.as_str().to_string();
            let cat_color = category.color();
            let is_safe = category.is_safe_to_delete();
            
            let mut lines = vec![
                Line::from(Span::styled(entry.name.clone(), Style::default().fg(TEXT).bold())),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Type       ", Style::default().fg(MUTED)),
                    Span::styled(if entry.is_dir { "📁 Directory" } else { "📄 File" }, Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Size       ", Style::default().fg(MUTED)),
                    Span::styled(humansize::format_size(entry.size, humansize::DECIMAL), Style::default().fg(ACCENT)),
                ]),
                Line::from(vec![
                    Span::styled("Category   ", Style::default().fg(MUTED)),
                    Span::styled(cat_name, Style::default().fg(cat_color)),
                ]),
                Line::from(vec![
                    Span::styled("Modified   ", Style::default().fg(MUTED)),
                    Span::styled(entry.modified.format("%Y-%m-%d %H:%M").to_string(), Style::default().fg(TEXT)),
                ]),
                Line::from(""),
            ];

            if entry.is_system {
                lines.push(Line::from(Span::styled("⚠ System file - Protected", Style::default().fg(DANGER))));
            } else if is_safe {
                lines.push(Line::from(Span::styled("✓ Safe to delete", Style::default().fg(SUCCESS))));
            } else {
                lines.push(Line::from(Span::styled("! Review before deleting", Style::default().fg(WARNING))));
            }

            lines.push(Line::from(""));
            let path_str = entry.path.to_string_lossy();
            let path_display = if path_str.len() > 35 {
                format!("...{}", &path_str[path_str.len()-32..])
            } else {
                path_str.to_string()
            };
            lines.push(Line::from(vec![
                Span::styled("Path: ", Style::default().fg(MUTED)),
                Span::styled(path_display, Style::default().fg(TEXT_DIM)),
            ]));

            lines
        } else {
            vec![Line::from(Span::styled("No selection", Style::default().fg(MUTED)))]
        }
    } else {
        vec![
            Line::from(Span::styled("No file selected", Style::default().fg(MUTED))),
            Line::from(""),
            Line::from(Span::styled("Use ↑↓ or mouse to navigate", Style::default().fg(TEXT_DIM))),
            Line::from(Span::styled("Space or click to mark files", Style::default().fg(TEXT_DIM))),
            Line::from(Span::styled("Enter to open folders", Style::default().fg(TEXT_DIM))),
        ]
    };

    let details_widget = Paragraph::new(details)
        .block(Block::default()
            .title(Span::styled(" 📋 Details ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .wrap(Wrap { trim: true });
    
    f.render_widget(details_widget, chunks[0]);

    // Recommendations section
    let rec_items: Vec<Line> = app
        .recommendations
        .iter()
        .take(6)
        .map(|r| Line::from(vec![
            Span::styled("💡 ", Style::default()),
            Span::styled(r, Style::default().fg(TEXT_DIM)),
        ]))
        .collect();

    let recommendations = Paragraph::new(rec_items)
        .block(Block::default()
            .title(Span::styled(" 💡 Recommendations ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))))
        .wrap(Wrap { trim: true });
    
    f.render_widget(recommendations, chunks[1]);
}

fn render_action_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let status_style = match app.state {
        AppState::Confirmation | AppState::SystemWarning => Style::default().fg(DANGER),
        AppState::Deleting => Style::default().fg(WARNING),
        _ => Style::default().fg(TEXT_DIM),
    };

    let status = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(&app.status_message, status_style),
    ]))
    .block(Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    
    f.render_widget(status, chunks[0]);

    let hints = if app.state == AppState::Confirmation || app.state == AppState::SystemWarning {
        vec![
            Span::styled("Y", Style::default().fg(SUCCESS)),
            Span::styled(" Confirm  ", Style::default().fg(MUTED)),
            Span::styled("N", Style::default().fg(DANGER)),
            Span::styled(" Cancel", Style::default().fg(MUTED)),
        ]
    } else {
        vec![
            Span::styled("?", Style::default().fg(ACCENT)),
            Span::styled(" Help ", Style::default().fg(MUTED)),
            Span::styled("Space", Style::default().fg(ACCENT)),
            Span::styled(" Mark ", Style::default().fg(MUTED)),
            Span::styled("d", Style::default().fg(DANGER)),
            Span::styled(" Del ", Style::default().fg(MUTED)),
            Span::styled("f", Style::default().fg(SUCCESS)),
            Span::styled(" All ", Style::default().fg(MUTED)),
            Span::styled("v", Style::default().fg(ACCENT)),
            Span::styled(" View ", Style::default().fg(MUTED)),
            Span::styled("q", Style::default().fg(ACCENT)),
            Span::styled(" Quit", Style::default().fg(MUTED)),
        ]
    };

    let hints_widget = Paragraph::new(Line::from(hints))
        .alignment(Alignment::Right)
        .block(Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(45, 45, 60))));
    
    f.render_widget(hints_widget, chunks[1]);
}
