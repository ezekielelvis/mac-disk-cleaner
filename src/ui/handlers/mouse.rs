#![allow(dead_code)]

use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use ratatui::layout::Rect;

/// Result of mouse event handling
pub enum MouseResult {
    None,
    Click(u16, u16),      // x, y coordinates
    ScrollUp,
    ScrollDown,
    DoubleClick(u16, u16),
    RightClick(u16, u16),
}

/// Process mouse events
pub fn process_mouse_event(event: MouseEvent) -> MouseResult {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            MouseResult::Click(event.column, event.row)
        }
        MouseEventKind::Down(MouseButton::Right) => {
            MouseResult::RightClick(event.column, event.row)
        }
        MouseEventKind::ScrollUp => MouseResult::ScrollUp,
        MouseEventKind::ScrollDown => MouseResult::ScrollDown,
        _ => MouseResult::None,
    }
}

/// Check if a click is within a given area
pub fn click_in_area(x: u16, y: u16, area: Rect) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

/// Calculate which list item was clicked given an area and click position
pub fn calculate_list_item_clicked(
    click_y: u16,
    list_area: Rect,
    scroll_offset: usize,
    item_height: usize,
) -> Option<usize> {
    if click_y < list_area.y + 1 || click_y >= list_area.y + list_area.height - 1 {
        return None; // Click on border
    }
    
    let relative_y = (click_y - list_area.y - 1) as usize; // -1 for top border
    let item_index = scroll_offset + relative_y / item_height;
    
    Some(item_index)
}

/// Calculate scroll position for home menu options
pub fn calculate_home_option_clicked(
    click_y: u16,
    options_area: Rect,
    item_height: usize,
) -> Option<usize> {
    if click_y < options_area.y + 1 || click_y >= options_area.y + options_area.height - 1 {
        return None;
    }
    
    let relative_y = (click_y - options_area.y - 1) as usize;
    Some(relative_y / item_height)
}
