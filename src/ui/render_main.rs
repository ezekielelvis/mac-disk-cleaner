use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};
use crate::analyzer::Analyzer;
use crate::ui::app::App;
use crate::ui::types::AppState;
use crate::ui::colors::*;

/// Render the main header with stats and storage bar
pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    // Title and stats
    let stats = if let Some(ref result) = app.scan_result {
        format!(
            "📄 {} files  📁 {} dirs  💾 {}  ✓ {} marked",
            result.total_files,
            result.total_dirs,
            humansize::format_size(result.total_size, humansize::DECIMAL),
            app.marked_for_deletion.len()
        )
    } else {
        String::new()
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("◉ ", Style::default().fg(ACCENT)),
        Span::styled("DISK CLEANER", Style::default().fg(TEXT).bold()),
        Span::styled("  ", Style::default()),
        Span::styled(stats, Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(header, chunks[0]);

    // Compact storage bar
    let storage = &app.storage_info;
    let usage = storage.usage_percent();
    let bar_color = if usage > 0.9 { DANGER } else if usage > 0.75 { WARNING } else { SUCCESS };
    
    let used = humansize::format_size(storage.used_space, humansize::DECIMAL);
    let total = humansize::format_size(storage.total_space, humansize::DECIMAL);
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(45, 45, 60)))
        .ratio(usage.min(1.0))
        .label(Span::styled(
            format!("{} / {} ({:.0}%)", used, total, usage * 100.0),
            Style::default().fg(TEXT)
        ));
    f.render_widget(gauge, chunks[1]);
}

/// Render breadcrumb navigation
pub fn render_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    let path_display = app.current_path.to_string_lossy();
    let truncated = if path_display.len() > 80 {
        format!("...{}", &path_display[path_display.len()-77..])
    } else {
        path_display.to_string()
    };

    let breadcrumb = Paragraph::new(Line::from(vec![
        Span::styled("📂 ", Style::default()),
        Span::styled(truncated, Style::default().fg(ACCENT)),
        if !app.navigation_stack.is_empty() {
            Span::styled("  ← Backspace to go back", Style::default().fg(MUTED))
        } else {
            Span::raw("")
        },
    ]));
    f.render_widget(breadcrumb, area);
}

/// Render system warning message
pub fn render_system_warning(f: &mut Frame, app: &App, area: Rect) {
    let warning_lines: Vec<Line> = app.system_warning_message
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(DANGER))))
        .collect();

    let warning = Paragraph::new(warning_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DANGER))
            .title(Span::styled(" ⚠️  DANGER ", Style::default().fg(DANGER).bold())))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    
    f.render_widget(warning, area);
}

/// Render the file list view
pub fn render_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let current_entries = app.get_current_entries();
    
    let items: Vec<ListItem> = current_entries
        .iter()
        .map(|(actual_idx, entry)| {
            let category = Analyzer::categorize_file(entry);
            let marked = if app.marked_for_deletion.contains(actual_idx) {
                Span::styled("● ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("○ ", Style::default().fg(MUTED))
            };
            
            let icon = if entry.is_system {
                Span::styled("⚙ ", Style::default().fg(DANGER))
            } else if entry.is_hidden {
                Span::styled("◌ ", Style::default().fg(MUTED))
            } else if entry.is_dir {
                Span::styled("▸ ", Style::default().fg(ACCENT))
            } else {
                Span::styled("  ", Style::default())
            };
            
            let name_style = if entry.is_system {
                Style::default().fg(DANGER).dim()
            } else if entry.is_dir {
                Style::default().fg(TEXT).bold()
            } else {
                Style::default().fg(TEXT)
            };

            let name_display = if entry.name.len() > 30 {
                format!("{}...", &entry.name[..27])
            } else {
                format!("{:<30}", entry.name)
            };
            
            let size_str = humansize::format_size(entry.size, humansize::DECIMAL);
            
            ListItem::new(Line::from(vec![
                marked,
                icon,
                Span::styled(name_display, name_style),
                Span::styled(format!("{:>10}", size_str), Style::default().fg(TEXT_DIM)),
                Span::styled(format!("  {}", category.as_str()), Style::default().fg(category.color())),
            ]))
        })
        .collect();

    let hidden_indicator = if app.show_hidden { "" } else { " (hidden filtered)" };
    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(
                format!(" Files{} ", hidden_indicator),
                Style::default().fg(TEXT_DIM)
            ))
            .borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::Rgb(55, 55, 75)))
        .highlight_symbol("  ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

/// Render category view
pub fn render_category_view(f: &mut Frame, app: &mut App, area: Rect) {
    let mut categories: Vec<_> = app.categories.iter().collect();
    categories.sort_by(|a, b| {
        let size_a: u64 = a.1.iter().map(|e| e.size).sum();
        let size_b: u64 = b.1.iter().map(|e| e.size).sum();
        size_b.cmp(&size_a)
    });

    let items: Vec<ListItem> = categories
        .iter()
        .map(|(category, entries)| {
            let total_size: u64 = entries.iter().map(|e| e.size).sum();
            let safe_indicator = if category.is_safe_to_delete() {
                Span::styled("✓ ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("! ", Style::default().fg(WARNING))
            };
            
            ListItem::new(Line::from(vec![
                safe_indicator,
                Span::styled(format!("{:<20}", category.as_str()), Style::default().fg(category.color())),
                Span::styled(format!("{:>6} items", entries.len()), Style::default().fg(TEXT_DIM)),
                Span::styled(format!("{:>12}", humansize::format_size(total_size, humansize::DECIMAL)), Style::default().fg(TEXT)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(" Categories ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::Rgb(55, 55, 75)))
        .highlight_symbol("  ");

    f.render_stateful_widget(list, area, &mut app.category_state);
}

/// Render sidebar with recommendations and details
pub fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Recommendations
    let rec_items: Vec<Line> = app
        .recommendations
        .iter()
        .take(5)
        .map(|r| Line::from(vec![
            Span::styled("  → ", Style::default().fg(WARNING)),
            Span::styled(r, Style::default().fg(TEXT_DIM)),
        ]))
        .collect();

    let recommendations = Paragraph::new(rec_items)
        .block(Block::default()
            .title(Span::styled(" 💡 Tips ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    
    f.render_widget(recommendations, chunks[0]);

    // Details panel
    let details = if let Some(visible_idx) = app.list_state.selected() {
        let current_entries = app.get_current_entries();
        if let Some((_, entry)) = current_entries.get(visible_idx) {
            let category = Analyzer::categorize_file(entry);
            let cat_str = category.as_str().to_string();
            let cat_color = category.color();
            let is_safe = category.is_safe_to_delete();
            let name = entry.name.clone();
            let size = entry.size;
            let is_dir = entry.is_dir;
            let is_system = entry.is_system;
            let modified = entry.modified.format("%Y-%m-%d").to_string();
            
            let mut lines = vec![
                Line::from(Span::styled(name, Style::default().fg(TEXT).bold())),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Size     ", Style::default().fg(MUTED)),
                    Span::styled(humansize::format_size(size, humansize::DECIMAL), Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Type     ", Style::default().fg(MUTED)),
                    Span::styled(if is_dir { "Directory" } else { "File" }, Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Category ", Style::default().fg(MUTED)),
                    Span::styled(cat_str, Style::default().fg(cat_color)),
                ]),
                Line::from(vec![
                    Span::styled("Modified ", Style::default().fg(MUTED)),
                    Span::styled(modified, Style::default().fg(TEXT)),
                ]),
            ];

            if is_system {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("⚠ System file - protected", Style::default().fg(DANGER))));
            }

            if is_safe {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("✓ Safe to delete", Style::default().fg(SUCCESS))));
            }

            lines
        } else {
            vec![Line::from(Span::styled("No selection", Style::default().fg(MUTED)))]
        }
    } else {
        vec![Line::from(Span::styled("No selection", Style::default().fg(MUTED)))]
    };

    let details_widget = Paragraph::new(details)
        .block(Block::default()
            .title(Span::styled(" Details ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    
    f.render_widget(details_widget, chunks[1]);
}

/// Render category detail view
pub fn render_category_detail_view(f: &mut Frame, app: &App, area: Rect) {
    if let Some(category) = app.selected_category {
        if let Some(entries) = app.categories.get(&category) {
            let items: Vec<ListItem> = entries
                .iter()
                .map(|entry| {
                    let icon = if entry.is_system {
                        Span::styled("⚙ ", Style::default().fg(DANGER))
                    } else if entry.is_dir {
                        Span::styled("▸ ", Style::default().fg(ACCENT))
                    } else {
                        Span::styled("  ", Style::default())
                    };
                    
                    let name_display = if entry.name.len() > 40 {
                        format!("{}...", &entry.name[..37])
                    } else {
                        entry.name.clone()
                    };
                    
                    ListItem::new(Line::from(vec![
                        icon,
                        Span::styled(format!("{:<42}", name_display), Style::default().fg(TEXT)),
                        Span::styled(
                            humansize::format_size(entry.size, humansize::DECIMAL),
                            Style::default().fg(TEXT_DIM)
                        ),
                    ]))
                })
                .collect();

            let total_size: u64 = entries.iter().map(|e| e.size).sum();
            let safe_text = if category.is_safe_to_delete() { "✓ Safe" } else { "! Review" };
            
            let list = List::new(items)
                .block(Block::default()
                    .title(Span::styled(
                        format!(" {} · {} · {} ", category.as_str(), humansize::format_size(total_size, humansize::DECIMAL), safe_text),
                        Style::default().fg(category.color())
                    ))
                    .borders(Borders::NONE))
                .highlight_style(Style::default().bg(Color::Rgb(55, 55, 75)));

            f.render_widget(list, area);
        }
    }
}

/// Render help screen
pub fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled("NAVIGATION", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑ ↓ j k    ", Style::default().fg(TEXT)),
            Span::styled("Move selection", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  → l Enter  ", Style::default().fg(TEXT)),
            Span::styled("Enter folder / View category", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ← Back     ", Style::default().fg(TEXT)),
            Span::styled("Go back", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("ACTIONS", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Space      ", Style::default().fg(TEXT)),
            Span::styled("Toggle mark", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  s          ", Style::default().fg(TEXT)),
            Span::styled("Mark safe items", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  a          ", Style::default().fg(TEXT)),
            Span::styled("Mark all (except system)", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  c          ", Style::default().fg(TEXT)),
            Span::styled("Clear marks", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  d          ", Style::default().fg(TEXT)),
            Span::styled("Delete marked", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("VIEW", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  v          ", Style::default().fg(TEXT)),
            Span::styled("Toggle file/category view", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  .          ", Style::default().fg(TEXT)),
            Span::styled("Toggle hidden files", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", Style::default().fg(TEXT)),
            Span::styled("Toggle help", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  q          ", Style::default().fg(TEXT)),
            Span::styled("Quit", Style::default().fg(MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled("INDICATORS", Style::default().fg(ACCENT).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(SUCCESS)),
            Span::styled("Safe to delete", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ! ", Style::default().fg(WARNING)),
            Span::styled("Review before deleting", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("  ⚙ ", Style::default().fg(DANGER)),
            Span::styled("System file - protected", Style::default().fg(MUTED)),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default()
            .title(Span::styled(" Help ", Style::default().fg(TEXT_DIM)))
            .borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    
    f.render_widget(help, area);
}

/// Render footer with status message and keyhints
pub fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let style = match app.state {
        AppState::Confirmation | AppState::SystemWarning => Style::default().fg(DANGER),
        AppState::Deleting => Style::default().fg(WARNING),
        _ => Style::default().fg(TEXT_DIM),
    };

    let keyhints = if app.state == AppState::Confirmation || app.state == AppState::SystemWarning {
        vec![]
    } else {
        vec![
            Span::styled("  ?", Style::default().fg(ACCENT)),
            Span::styled(" help  ", Style::default().fg(MUTED)),
            Span::styled("Space", Style::default().fg(ACCENT)),
            Span::styled(" mark  ", Style::default().fg(MUTED)),
            Span::styled("d", Style::default().fg(ACCENT)),
            Span::styled(" delete  ", Style::default().fg(MUTED)),
            Span::styled("v", Style::default().fg(ACCENT)),
            Span::styled(" view  ", Style::default().fg(MUTED)),
            Span::styled("q", Style::default().fg(ACCENT)),
            Span::styled(" quit", Style::default().fg(MUTED)),
        ]
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(&app.status_message, style),
        Span::raw("  "),
    ].into_iter().chain(keyhints).collect::<Vec<_>>()))
    .alignment(Alignment::Left);
    
    f.render_widget(footer, area);
}
