use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use crate::ui::colors::*;

/// Render path input modal dialog
pub fn render_path_input(f: &mut Frame, input: &str, _cursor_pos: usize, suggestions: &[String]) {
    let area = f.area();
    
    // Center the modal - responsive to screen size
    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 14.min(area.height.saturating_sub(4));
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;
    
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

/// Render help overlay modal
pub fn render_help_overlay(f: &mut Frame, area: Rect) {
    // Center the help modal - responsive
    let modal_width = 58.min(area.width.saturating_sub(4));
    let modal_height = 32.min(area.height.saturating_sub(2));
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;
    
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
        Line::from(vec![
            Span::styled("    h           ", Style::default().fg(TEXT)),
            Span::styled("Return to home screen", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    Mouse Click ", Style::default().fg(TEXT)),
            Span::styled("Select item / Scroll", Style::default().fg(MUTED)),
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
        Line::from(vec![
            Span::styled("    /           ", Style::default().fg(TEXT)),
            Span::styled("Search files by name", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("    o           ", Style::default().fg(TEXT)),
            Span::styled("Cycle sort mode", Style::default().fg(MUTED)),
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
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;
    
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
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;
    
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

/// Render search input dialog
pub fn render_search_dialog(f: &mut Frame, search_query: &str, area: Rect) {
    let modal_width = 50.min(area.width.saturating_sub(4));
    let modal_height = 5;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = 2; // Near top of screen
    
    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);
    
    f.render_widget(Clear, modal_area);

    let input_display = if search_query.is_empty() {
        "Type to search..."
    } else {
        search_query
    };
    
    let input_style = if search_query.is_empty() {
        Style::default().fg(MUTED)
    } else {
        Style::default().fg(TEXT)
    };

    let dialog = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  🔍 ", Style::default().fg(ACCENT)),
            Span::styled(input_display, input_style),
            Span::styled("█", Style::default().fg(ACCENT)), // Cursor
        ]),
    ])
    .block(Block::default()
        .title(Span::styled(" Search Files ", Style::default().fg(ACCENT)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Rgb(30, 30, 45))));
    
    f.render_widget(dialog, modal_area);
}
