#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyModifiers};
use std::path::PathBuf;

/// Handle home screen keyboard input
pub fn handle_home_input(
    key: KeyCode,
    _modifiers: KeyModifiers,
    selected_option: &mut usize,
    options_len: usize,
    _custom_path: &str,
) -> HomeInputResult {
    match key {
        KeyCode::Char('q') => HomeInputResult::Quit,
        KeyCode::Up | KeyCode::Char('k') => {
            if *selected_option > 0 {
                *selected_option -= 1;
            }
            HomeInputResult::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if *selected_option < options_len - 1 {
                *selected_option += 1;
            }
            HomeInputResult::None
        }
        KeyCode::Enter => HomeInputResult::StartScan,
        KeyCode::Char('p') => HomeInputResult::OpenPathInput,
        KeyCode::Char('+') | KeyCode::Char('=') => HomeInputResult::IncreaseMinSize,
        KeyCode::Char('-') => HomeInputResult::DecreaseMinSize,
        KeyCode::Char('d') => HomeInputResult::CycleDepth,
        KeyCode::Char('.') => HomeInputResult::ToggleHidden,
        _ => HomeInputResult::None,
    }
}

pub enum HomeInputResult {
    None,
    Quit,
    StartScan,
    OpenPathInput,
    IncreaseMinSize,
    DecreaseMinSize,
    CycleDepth,
    ToggleHidden,
}

/// Handle path input keyboard input
pub fn handle_path_input(
    key: KeyCode,
    path_input: &mut String,
    cursor_pos: &mut usize,
    suggestions: &[String],
) -> PathInputResult {
    match key {
        KeyCode::Esc => PathInputResult::Cancel,
        KeyCode::Enter => {
            if !path_input.is_empty() && PathBuf::from(&*path_input).exists() {
                PathInputResult::Confirm(path_input.clone())
            } else {
                PathInputResult::Cancel
            }
        }
        KeyCode::Tab => {
            if !suggestions.is_empty() {
                *path_input = suggestions[0].clone();
                *cursor_pos = path_input.len();
            }
            PathInputResult::UpdateSuggestions
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                path_input.remove(*cursor_pos - 1);
                *cursor_pos -= 1;
            }
            PathInputResult::UpdateSuggestions
        }
        KeyCode::Delete => {
            if *cursor_pos < path_input.len() {
                path_input.remove(*cursor_pos);
            }
            PathInputResult::UpdateSuggestions
        }
        KeyCode::Left => {
            *cursor_pos = cursor_pos.saturating_sub(1);
            PathInputResult::None
        }
        KeyCode::Right => {
            *cursor_pos = (*cursor_pos + 1).min(path_input.len());
            PathInputResult::None
        }
        KeyCode::Char(c) => {
            path_input.insert(*cursor_pos, c);
            *cursor_pos += 1;
            PathInputResult::UpdateSuggestions
        }
        _ => PathInputResult::None,
    }
}

pub enum PathInputResult {
    None,
    Cancel,
    Confirm(String),
    UpdateSuggestions,
}

/// Handle viewing/browsing screen input
pub fn handle_viewing_input(
    key: KeyCode,
    _modifiers: KeyModifiers,
) -> ViewingInputResult {
    match key {
        KeyCode::Char('q') => ViewingInputResult::Quit,
        KeyCode::Down | KeyCode::Char('j') => ViewingInputResult::NextItem,
        KeyCode::Up | KeyCode::Char('k') => ViewingInputResult::PreviousItem,
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => ViewingInputResult::Enter,
        KeyCode::Left | KeyCode::Backspace => ViewingInputResult::Back,
        KeyCode::Char(' ') => ViewingInputResult::ToggleMark,
        KeyCode::Char('d') => ViewingInputResult::Delete,
        KeyCode::Char('?') => ViewingInputResult::ToggleHelp,
        KeyCode::Char('v') => ViewingInputResult::SwitchView,
        KeyCode::Char('.') => ViewingInputResult::ToggleHidden,
        KeyCode::Char('a') => ViewingInputResult::SelectAll,
        KeyCode::Char('s') => ViewingInputResult::SelectSafe,
        KeyCode::Char('c') => ViewingInputResult::ClearSelection,
        KeyCode::Char('h') => ViewingInputResult::GoHome,
        KeyCode::Char('f') => ViewingInputResult::OpenAllFiles,
        KeyCode::Char('/') => ViewingInputResult::OpenSearch,
        KeyCode::Esc => ViewingInputResult::Escape,
        KeyCode::PageDown => ViewingInputResult::PageDown,
        KeyCode::PageUp => ViewingInputResult::PageUp,
        _ => ViewingInputResult::None,
    }
}

pub enum ViewingInputResult {
    None,
    Quit,
    NextItem,
    PreviousItem,
    Enter,
    Back,
    ToggleMark,
    Delete,
    ToggleHelp,
    SwitchView,
    ToggleHidden,
    SelectAll,
    SelectSafe,
    ClearSelection,
    GoHome,
    OpenAllFiles,
    OpenSearch,
    Escape,
    PageDown,
    PageUp,
}

/// Handle all files screen input
pub fn handle_all_files_input(key: KeyCode) -> AllFilesInputResult {
    match key {
        KeyCode::Char('q') => AllFilesInputResult::Quit,
        KeyCode::Down | KeyCode::Char('j') => AllFilesInputResult::NextItem,
        KeyCode::Up | KeyCode::Char('k') => AllFilesInputResult::PreviousItem,
        KeyCode::Char(' ') => AllFilesInputResult::ToggleMark,
        KeyCode::Char('d') => AllFilesInputResult::Delete,
        KeyCode::Char('o') => AllFilesInputResult::CycleSort,
        KeyCode::Char('t') => AllFilesInputResult::CycleFilter,
        KeyCode::Char('/') => AllFilesInputResult::OpenSearch,
        KeyCode::Char('s') => AllFilesInputResult::SelectSafe,
        KeyCode::Char('a') => AllFilesInputResult::SelectAll,
        KeyCode::Char('c') => AllFilesInputResult::ClearSelection,
        KeyCode::Enter => AllFilesInputResult::OpenItem,
        KeyCode::Esc => AllFilesInputResult::Back,
        KeyCode::PageDown => AllFilesInputResult::PageDown,
        KeyCode::PageUp => AllFilesInputResult::PageUp,
        _ => AllFilesInputResult::None,
    }
}

pub enum AllFilesInputResult {
    None,
    Quit,
    NextItem,
    PreviousItem,
    ToggleMark,
    Delete,
    CycleSort,
    CycleFilter,
    OpenSearch,
    SelectSafe,
    SelectAll,
    ClearSelection,
    OpenItem,
    Back,
    PageDown,
    PageUp,
}

/// Handle confirmation dialog input
pub fn handle_confirmation_input(key: KeyCode) -> ConfirmationResult {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => ConfirmationResult::Confirm,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => ConfirmationResult::Cancel,
        _ => ConfirmationResult::None,
    }
}

pub enum ConfirmationResult {
    None,
    Confirm,
    Cancel,
}

/// Handle search input
pub fn handle_search_input(key: KeyCode, search_query: &mut String) -> SearchInputResult {
    match key {
        KeyCode::Esc => {
            search_query.clear();
            SearchInputResult::Close
        }
        KeyCode::Enter => SearchInputResult::Close,
        KeyCode::Backspace => {
            search_query.pop();
            SearchInputResult::Update
        }
        KeyCode::Char(c) => {
            search_query.push(c);
            SearchInputResult::Update
        }
        _ => SearchInputResult::None,
    }
}

pub enum SearchInputResult {
    None,
    Close,
    Update,
}
