use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
};
use crate::analyzer::Analyzer;
use crate::ui::app::App;
use crate::ui::types::{AppState, ViewMode};
use crate::ui::colors::*;

/// Render the scan complete summary screen with quick actions
pub fn render_scan_complete(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
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

        // Files scanned
        let files_block = render_stat_block(
            "📄 FILES",
            &format!("{}", result.total_files),
            ACCENT
        );
        f.render_widget(files_block, stats_chunks[0]);

        // Directories
        let dirs_block = render_stat_block(
            "📁 DIRECTORIES",
            &format!("{}", result.total_dirs),
            SUCCESS
        );
        f.render_widget(dirs_block, stats_chunks[1]);

        // Total size (use actual disk space, not scanned size)
        let total_block = render_stat_block(
            "💾 DISK USED",
            &humansize::format_size(app.storage_info.used_space, humansize::DECIMAL),
            WARNING
        );
        f.render_widget(total_block, stats_chunks[2]);

        // Potential savings
        let savings_block = render_stat_block(
            "🎯 SAFE TO DELETE",
            &humansize::format_size(safe_size, humansize::DECIMAL),
            Color::Rgb(34, 197, 94)
        );
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

    // Quick actions
    let actions = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(MUTED)),
            Span::styled("Enter", Style::default().fg(ACCENT).bold()),
            Span::styled(" to Browse & Delete Files    ", Style::default().fg(MUTED)),
            Span::styled("s", Style::default().fg(SUCCESS).bold()),
            Span::styled(" Select All Safe    ", Style::default().fg(MUTED)),
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

fn render_stat_block(label: &str, value: &str, color: Color) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(label.to_string(), Style::default().fg(MUTED))),
        Line::from(""),
        Line::from(Span::styled(value.to_string(), Style::default().fg(color).bold())),
    ])
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(45, 45, 60))))
}

/// Render the enhanced results view with better navigation
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

    // Title and summary stats
    let stats = if let Some(ref result) = app.scan_result {
        let marked_size: u64 = app.marked_for_deletion.iter()
            .filter_map(|&i| result.entries.get(i))
            .map(|e| e.size)
            .sum();
        
        format!(
            "📄 {} files  📁 {} dirs  💾 {}  ◌ {} hidden  ⚙ {} system  │  ✓ {} selected ({})",
            result.total_files,
            result.total_dirs,
            humansize::format_size(result.total_size, humansize::DECIMAL),
            result.hidden_count,
            result.system_count,
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

    // View mode tabs
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

    // Breadcrumb
    let path_display = app.current_path.to_string_lossy();
    let truncated = if path_display.len() > 60 {
        format!("...{}", &path_display[path_display.len()-57..])
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

    // Left panel - file list, categories, or category detail
    if app.state == AppState::CategoryView {
        render_category_detail(f, app, chunks[0]);
    } else {
        match app.current_view {
            ViewMode::AllFiles => render_enhanced_file_list(f, app, chunks[0]),
            ViewMode::Categories => render_enhanced_categories(f, app, chunks[0]),
        }
    }

    // Right panel - details and recommendations
    render_details_panel(f, app, chunks[1]);
}

fn render_enhanced_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let current_entries = app.get_current_entries();
    let total_items = current_entries.len();
    
    let items: Vec<ListItem> = current_entries
        .iter()
        .enumerate()
        .map(|(_visible_idx, (actual_idx, entry))| {
            let category = Analyzer::categorize_file(entry);
            
            // Selection marker with animation effect
            let marked = if app.marked_for_deletion.contains(actual_idx) {
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

    let hidden_text = if !app.show_hidden { " (hidden filtered)" } else { "" };
    
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

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_enhanced_categories(f: &mut Frame, app: &mut App, area: Rect) {
    let mut categories: Vec<_> = app.categories.iter().collect();
    categories.sort_by(|a, b| {
        let size_a: u64 = a.1.iter().map(|e| e.size).sum();
        let size_b: u64 = b.1.iter().map(|e| e.size).sum();
        size_b.cmp(&size_a)
    });

    // Calculate total for percentage
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
            
            // Visual bar for size proportion
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
            let safe_text = if category.is_safe_to_delete() { "✓ Safe to delete" } else { "! Review items" };
            
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
        }
    } else {
        let empty = Paragraph::new("No category selected")
            .style(Style::default().fg(MUTED))
            .block(Block::default()
                .title(" Category Detail ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
        f.render_widget(empty, area);
    }
}

fn render_details_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),  // Selected item details
            Constraint::Percentage(50),  // Recommendations
        ])
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

            // Status indicators
            if entry.is_system {
                lines.push(Line::from(Span::styled("⚠ System file - Protected", Style::default().fg(DANGER))));
            } else if is_safe {
                lines.push(Line::from(Span::styled("✓ Safe to delete", Style::default().fg(SUCCESS))));
            } else {
                lines.push(Line::from(Span::styled("! Review before deleting", Style::default().fg(WARNING))));
            }

            // Path info
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
            Line::from(Span::styled("Use ↑↓ to navigate", Style::default().fg(TEXT_DIM))),
            Line::from(Span::styled("Space to mark files", Style::default().fg(TEXT_DIM))),
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

    // Status message
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

    // Key hints
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
            Span::styled(" Delete ", Style::default().fg(MUTED)),
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

/// Render the enhanced help overlay
pub fn render_help_overlay(f: &mut Frame, area: Rect) {
    // Center the help modal
    let modal_width = 55.min(area.width.saturating_sub(4));
    let modal_height = 28.min(area.height.saturating_sub(2));
    let modal_x = (area.width - modal_width) / 2;
    let modal_y = (area.height - modal_height) / 2;
    
    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);
    
    f.render_widget(Clear, modal_area);

    let help_text = vec![
        Line::from(Span::styled("  NAVIGATION", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ↑ ↓ j k     ", Style::default().fg(TEXT)),
            Span::styled("Move selection", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    → l Enter   ", Style::default().fg(TEXT)),
            Span::styled("Enter folder / View category", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    ← Backspace ", Style::default().fg(TEXT)),
            Span::styled("Go back", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  SELECTION", Style::default().fg(SUCCESS).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("    Space       ", Style::default().fg(TEXT)),
            Span::styled("Toggle mark for deletion", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    s           ", Style::default().fg(TEXT)),
            Span::styled("Select all safe items", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    a           ", Style::default().fg(TEXT)),
            Span::styled("Select all (except system)", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    c           ", Style::default().fg(TEXT)),
            Span::styled("Clear all selections", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  ACTIONS", Style::default().fg(DANGER).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("    d           ", Style::default().fg(TEXT)),
            Span::styled("Delete selected items", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    v           ", Style::default().fg(TEXT)),
            Span::styled("Toggle file/category view", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    .           ", Style::default().fg(TEXT)),
            Span::styled("Toggle hidden files", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  INDICATORS", Style::default().fg(WARNING).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ✓ ", Style::default().fg(SUCCESS)),
            Span::styled("Safe to delete", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    ! ", Style::default().fg(WARNING)),
            Span::styled("Review before deleting", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    ⚙ ", Style::default().fg(DANGER)),
            Span::styled("System file - protected", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    Press ", Style::default().fg(MUTED)),
            Span::styled("?", Style::default().fg(ACCENT)),
            Span::styled(" or ", Style::default().fg(MUTED)),
            Span::styled("Esc", Style::default().fg(ACCENT)),
            Span::styled(" to close", Style::default().fg(MUTED)),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default()
            .title(Span::styled(" ⌨ Keyboard Shortcuts ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(Color::Rgb(25, 25, 40))))
        .wrap(Wrap { trim: true });
    
    f.render_widget(help, modal_area);
}

/// Render confirmation dialog
pub fn render_confirmation_dialog(f: &mut Frame, message: &str, area: Rect) {
    let modal_width = 50.min(area.width.saturating_sub(4));
    let modal_height = 8;
    let modal_x = (area.width - modal_width) / 2;
    let modal_y = (area.height - modal_height) / 2;
    
    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);
    
    f.render_widget(Clear, modal_area);

    let dialog = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("[Y]", Style::default().fg(SUCCESS).bold()),
            Span::styled(" Confirm    ", Style::default().fg(MUTED)),
            Span::styled("[N]", Style::default().fg(DANGER).bold()),
            Span::styled(" Cancel", Style::default().fg(MUTED)),
        ]),
    ])
    .block(Block::default()
        .title(Span::styled(" ⚠ Confirm Action ", Style::default().fg(WARNING)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(WARNING))
        .style(Style::default().bg(Color::Rgb(30, 30, 45))))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    
    f.render_widget(dialog, modal_area);
}

/// Render system warning dialog
pub fn render_system_warning_dialog(f: &mut Frame, message: &str, area: Rect) {
    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 14;
    let modal_x = (area.width - modal_width) / 2;
    let modal_y = (area.height - modal_height) / 2;
    
    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);
    
    f.render_widget(Clear, modal_area);

    let lines: Vec<Line> = message
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(TEXT))))
        .collect();

    let dialog = Paragraph::new(lines)
        .block(Block::default()
            .title(Span::styled(" 🛑 DANGER - System Files ", Style::default().fg(DANGER).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DANGER))
            .style(Style::default().bg(Color::Rgb(40, 20, 20))))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    f.render_widget(dialog, modal_area);
}
