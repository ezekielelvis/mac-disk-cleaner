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

    // Animated scanning indicator
    let scan_frames = ["◜", "◠", "◝", "◞", "◡", "◟"];
    let scan_indicator = scan_frames[((frame_count / 6) % 6) as usize];

    // Split path into segments and build tree
    let segments: Vec<&str> = path_display.split('/').filter(|s| !s.is_empty()).collect();
    
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", scan_indicator), Style::default().fg(ACCENT)),
            Span::styled("Scanning Directory Tree", Style::default().fg(TEXT).bold()),
        ]),
    ];
    
    if segments.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Initializing...", Style::default().fg(MUTED))));
    } else {
        // Colors for different depth levels - creates a gradient effect
        let depth_colors = [
            Color::Rgb(156, 163, 175),  // Gray
            Color::Rgb(129, 140, 248),  // Indigo light
            Color::Rgb(167, 139, 250),  // Purple
            Color::Rgb(192, 132, 252),  // Purple light
            Color::Rgb(139, 233, 253),  // Cyan
            Color::Rgb(94, 234, 212),   // Teal
            Color::Rgb(134, 239, 172),  // Green
            Color::Rgb(253, 224, 71),   // Yellow
        ];
        
        // Show root
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("/", Style::default().fg(TEXT).bold()),
        ]));
        
        // Build continuous tree structure - show ALL segments
        let max_display = (area.height as usize).saturating_sub(5).min(segments.len());
        let start_idx = if segments.len() > max_display { segments.len() - max_display } else { 0 };
        
        // Show ellipsis if path was truncated (at the top)
        if start_idx > 0 {
            lines.push(Line::from(vec![
                Span::styled("  │", Style::default().fg(Color::Rgb(75, 75, 95))),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ├─ ", Style::default().fg(Color::Rgb(75, 75, 95))),
                Span::styled(format!("... {} more levels ...", start_idx), Style::default().fg(MUTED).italic()),
            ]));
        }
        
        for (display_i, i) in (start_idx..segments.len()).enumerate() {
            let segment = segments[i];
            let is_last = i == segments.len() - 1;
            let color_idx = display_i % depth_colors.len();
            let segment_color = depth_colors[color_idx];
            
            // Create continuous tree line
            let mut line_spans = vec![Span::styled("  ", Style::default())];
            
            // Vertical connector line
            if !is_last {
                line_spans.push(Span::styled("├─ ", Style::default().fg(Color::Rgb(75, 75, 95))));
            } else {
                line_spans.push(Span::styled("└─ ", Style::default().fg(ACCENT)));
            }
            
            // Truncate long segment names
            let display_name = if segment.len() > 25 {
                format!("{}...", &segment[..22])
            } else {
                segment.to_string()
            };
            
            if is_last {
                // Active directory - highlighted with animation
                line_spans.push(Span::styled(display_name, Style::default().fg(ACCENT).bold()));
                
                // Animated scanning indicator
                let dots = match (frame_count / 8) % 4 {
                    0 => " ·",
                    1 => " ··",
                    2 => " ···",
                    _ => "",
                };
                line_spans.push(Span::styled(dots, Style::default().fg(SUCCESS)));
            } else {
                line_spans.push(Span::styled(display_name, Style::default().fg(segment_color)));
            }
            
            lines.push(Line::from(line_spans));
            
            // Add connecting vertical line for non-last items
            if !is_last && display_i < max_display - 1 {
                lines.push(Line::from(vec![
                    Span::styled("  │", Style::default().fg(Color::Rgb(55, 55, 75))),
                ]));
            }
        }
    }
    
    // Pad remaining lines
    while lines.len() < area.height as usize {
        lines.push(Line::from(""));
    }

    let path_widget = Paragraph::new(lines)
        .block(Block::default()
            .title(Span::styled(" Scan Path ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    
    f.render_widget(path_widget, area);
}

fn render_category_breakdown(f: &mut Frame, category_sizes: &std::collections::HashMap<String, u64>, total_size: u64, area: Rect, frame_count: u32) {
    // Distinct, vibrant colors for each category - no duplicates
    let category_colors: Vec<(&str, Color)> = vec![
        ("Cache", Color::Rgb(251, 191, 36)),       // Amber
        ("Temp Files", Color::Rgb(239, 68, 68)),   // Red
        ("Large Files", Color::Rgb(168, 85, 247)), // Purple
        ("Old Files", Color::Rgb(6, 182, 212)),    // Cyan
        ("Duplicate Names", Color::Rgb(59, 130, 246)), // Blue
        ("Log Files", Color::Rgb(245, 158, 11)),   // Orange
        ("Build Artifacts", Color::Rgb(236, 72, 153)), // Pink
        ("node_modules", Color::Rgb(139, 92, 246)), // Violet
        ("Package Cache", Color::Rgb(20, 184, 166)), // Teal
        ("Hidden Files", Color::Rgb(107, 114, 128)), // Gray
        ("System Files", Color::Rgb(220, 38, 38)), // Dark Red
        ("Library Files", Color::Rgb(96, 165, 250)), // Light Blue
        ("Downloads", Color::Rgb(34, 197, 94)),    // Green
        ("Documents", Color::Rgb(226, 232, 240)),  // Light
        ("Media", Color::Rgb(132, 204, 22)),       // Lime
        ("Archives", Color::Rgb(234, 179, 8)),     // Yellow
        ("Regular", Color::Rgb(148, 163, 184)),    // Slate
    ];
    
    // Sort categories by size (largest first)
    let mut sorted_categories: Vec<(&String, &u64)> = category_sizes.iter().collect();
    sorted_categories.sort_by(|a, b| b.1.cmp(a.1));
    
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Category Breakdown", Style::default().fg(TEXT).bold()),
        ]),
        Line::from(""),
    ];
    
    if sorted_categories.is_empty() || total_size == 0 {
        lines.push(Line::from(Span::styled("  Scanning...", Style::default().fg(MUTED))));
    } else {
        // Calculate bar width based on available space (wider bars)
        let bar_width = (area.width as usize).saturating_sub(30).min(35).max(15);
        
        // Build a segmented horizontal bar showing proportions
        let mut bar_spans: Vec<Span> = vec![Span::styled(" ", Style::default())];
        
        for (cat_name, size) in sorted_categories.iter().take(6) {
            let size_val = **size;
            let percentage = if total_size > 0 { size_val as f64 / total_size as f64 } else { 0.0 };
            if percentage >= 0.02 {  // Only show categories with >= 2%
                let segment_width = ((percentage) * bar_width as f64).max(1.0) as usize;
                
                // Find color for this category
                let color = category_colors.iter()
                    .find(|(name, _)| {
                        let search = name.to_lowercase();
                        cat_name.to_lowercase().contains(&search) || 
                        search.contains(&cat_name.to_lowercase().replace("🗑️ ", "").replace("🌡️ ", "").replace("📦 ", "").replace("📅 ", "").replace("👯 ", "").replace("📜 ", "").replace("🔨 ", "").replace("📥 ", "").replace("👁️ ", "").replace("⚙️ ", "").replace("📚 ", "").replace("⬇️ ", "").replace("📄 ", "").replace("🎬 ", "").replace("🗜️ ", "").replace("📁 ", ""))
                    })
                    .map(|(_, c)| *c)
                    .unwrap_or(TEXT_DIM);
                
                bar_spans.push(Span::styled(
                    "█".repeat(segment_width),
                    Style::default().fg(color)
                ));
            }
        }
        
        // Fill remaining with background
        let used_width: usize = bar_spans.iter().map(|s| s.content.len()).sum();
        if used_width < bar_width + 1 {
            bar_spans.push(Span::styled(
                "░".repeat(bar_width + 1 - used_width),
                Style::default().fg(Color::Rgb(45, 45, 60))
            ));
        }
        
        lines.push(Line::from(bar_spans));
        lines.push(Line::from(""));
        
        // Animated scanning indicator
        let scan_frames = ["◜", "◠", "◝", "◞", "◡", "◟"];
        let scan_char = scan_frames[((frame_count / 6) % 6) as usize];
        
        // List categories with sizes - cleaner format without heavy icons
        let max_items = (area.height.saturating_sub(6)) as usize;
        
        for (i, (cat_name, size)) in sorted_categories.iter().take(max_items).enumerate() {
            let size_val = **size;
            let percentage = if total_size > 0 { size_val as f64 / total_size as f64 * 100.0 } else { 0.0 };
            
            // Clean category name - remove emoji icons
            let clean_name = cat_name
                .replace("🗑️ ", "")
                .replace("🌡️ ", "")
                .replace("📦 ", "")
                .replace("📅 ", "")
                .replace("👯 ", "")
                .replace("📜 ", "")
                .replace("🔨 ", "")
                .replace("📥 ", "")
                .replace("👁️ ", "")
                .replace("⚙️ ", "")
                .replace("📚 ", "")
                .replace("⬇️ ", "")
                .replace("📄 ", "")
                .replace("🎬 ", "")
                .replace("🗜️ ", "")
                .replace("📁 ", "");
            
            // Find color for this category
            let color = category_colors.iter()
                .find(|(name, _)| {
                    let search = name.to_lowercase();
                    clean_name.to_lowercase().contains(&search) || search.contains(&clean_name.to_lowercase())
                })
                .map(|(_, c)| *c)
                .unwrap_or(TEXT_DIM);
            
            let size_str = humansize::format_size(size_val, humansize::DECIMAL);
            
            // Truncate category name if needed
            let display_name = if clean_name.len() > 14 {
                format!("{}...", &clean_name[..11])
            } else {
                format!("{:<14}", clean_name)
            };
            
            // Animated indicator for top category only
            let indicator = if i == 0 { 
                Span::styled(format!(" {} ", scan_char), Style::default().fg(SUCCESS))
            } else {
                Span::styled("   ", Style::default())
            };
            
            // Mini bar for each category
            let mini_bar_width = (percentage / 100.0 * 8.0).max(1.0).min(8.0) as usize;
            let mini_bar = "▓".repeat(mini_bar_width) + &"░".repeat(8 - mini_bar_width);
            
            lines.push(Line::from(vec![
                indicator,
                Span::styled(mini_bar, Style::default().fg(color)),
                Span::styled(" ", Style::default()),
                Span::styled(display_name, Style::default().fg(TEXT)),
                Span::styled(format!("{:>8}", size_str), Style::default().fg(TEXT_DIM)),
                Span::styled(format!(" {:>5.1}%", percentage), Style::default().fg(color)),
            ]));
        }
        
        // Show "other" if there are more categories
        if sorted_categories.len() > max_items {
            let other_count = sorted_categories.len() - max_items;
            let other_size: u64 = sorted_categories.iter().skip(max_items).map(|(_, s)| **s).sum();
            let other_percentage = if total_size > 0 { other_size as f64 / total_size as f64 * 100.0 } else { 0.0 };
            
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled("········ ", Style::default().fg(MUTED)),
                Span::styled(format!("+{:<6} more", other_count), Style::default().fg(TEXT_DIM)),
                Span::styled(format!("{:>8}", humansize::format_size(other_size, humansize::DECIMAL)), Style::default().fg(TEXT_DIM)),
                Span::styled(format!(" {:>5.1}%", other_percentage), Style::default().fg(MUTED)),
            ]));
        }
    }

    let category_widget = Paragraph::new(lines)
        .block(Block::default()
            .title(Span::styled(" Categories Found ", Style::default().fg(TEXT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 75))));
    
    f.render_widget(category_widget, area);
}
