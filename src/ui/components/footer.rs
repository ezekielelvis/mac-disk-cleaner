use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::ui::colors::*;

/// Render home screen footer with key hints
pub fn render_home_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓/jk", Style::default().fg(ACCENT)),
        Span::styled(" Select  ", Style::default().fg(MUTED)),
        Span::styled("Enter", Style::default().fg(ACCENT)),
        Span::styled(" Start  ", Style::default().fg(MUTED)),
        Span::styled("p", Style::default().fg(ACCENT)),
        Span::styled(" Path  ", Style::default().fg(MUTED)),
        Span::styled("+/-", Style::default().fg(ACCENT)),
        Span::styled(" Size  ", Style::default().fg(MUTED)),
        Span::styled("Mouse", Style::default().fg(ACCENT)),
        Span::styled(" Click  ", Style::default().fg(MUTED)),
        Span::styled("q", Style::default().fg(ACCENT)),
        Span::styled(" Quit", Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}

/// Render scanning screen footer
pub fn render_scan_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Q", Style::default().fg(DANGER)),
        Span::styled(" Cancel scan  ", Style::default().fg(MUTED)),
        Span::styled("↑↓", Style::default().fg(ACCENT)),
        Span::styled(" Scroll  ", Style::default().fg(MUTED)),
        Span::styled("•", Style::default().fg(MUTED)),
        Span::styled("  Scan will complete automatically", Style::default().fg(TEXT_DIM)),
    ]))
    .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}

/// Render all files screen footer
pub fn render_all_files_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓/jk", Style::default().fg(ACCENT)),
        Span::styled(" Move  ", Style::default().fg(MUTED)),
        Span::styled("Space", Style::default().fg(ACCENT)),
        Span::styled(" Select  ", Style::default().fg(MUTED)),
        Span::styled("Enter", Style::default().fg(SUCCESS)),
        Span::styled(" Open  ", Style::default().fg(MUTED)),
        Span::styled("d", Style::default().fg(DANGER)),
        Span::styled(" Delete  ", Style::default().fg(MUTED)),
        Span::styled("s", Style::default().fg(SUCCESS)),
        Span::styled(" Safe  ", Style::default().fg(MUTED)),
        Span::styled("/", Style::default().fg(ACCENT)),
        Span::styled(" Search  ", Style::default().fg(MUTED)),
        Span::styled("Esc", Style::default().fg(WARNING)),
        Span::styled(" Back  ", Style::default().fg(MUTED)),
        Span::styled("q", Style::default().fg(ACCENT)),
        Span::styled(" Quit", Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}

/// Render a generic footer with custom key hints
pub fn render_footer_with_hints(f: &mut Frame, area: Rect, hints: Vec<(&str, &str)>) {
    let mut spans = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(*key, Style::default().fg(ACCENT)));
        spans.push(Span::styled(format!(" {}", desc), Style::default().fg(MUTED)));
    }
    
    let footer = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}
