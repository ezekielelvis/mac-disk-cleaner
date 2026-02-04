#![allow(dead_code)]

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::ui::colors::*;

/// Render the animated logo for the home screen
pub fn render_logo(f: &mut Frame, area: Rect, frame_count: u32) {
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

/// Render a scan header with animated indicator
pub fn render_scan_header(f: &mut Frame, area: Rect, frame_count: u32) {
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

/// Render a simple page header
pub fn render_page_header(f: &mut Frame, area: Rect, title: &str, subtitle: &str) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("◉ ", Style::default().fg(ACCENT)),
        Span::styled(title, Style::default().fg(TEXT).bold()),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(55, 55, 75))),
        Span::styled(subtitle, Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(header, area);
}
