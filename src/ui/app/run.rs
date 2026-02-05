use super::state::App;
use super::scan::run_scan;
use super::super::types::*;
use super::super::screens::{render_home, render_scanning_enhanced, render_results_view, render_scan_details};
use super::super::components::{render_path_input, render_help_overlay, render_confirmation_dialog, render_system_warning_dialog};
use super::super::handlers::{process_mouse_event, MouseResult};
use crate::analyzer::Analyzer;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::Clear,
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;

pub async fn run_app(scan_path: PathBuf, min_size: u64, depth: usize) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(scan_path.clone());
    app.home_menu.min_size_mb = min_size;
    app.home_menu.max_depth = depth;
    
    let result = run_main_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn run_main_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        app.frame_count = app.frame_count.wrapping_add(1);
        
        terminal.draw(|f| render_ui(f, app))?;
        
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            
            if let Event::Mouse(mouse_event) = event {
                handle_mouse_event(terminal, app, mouse_event).await?;
                continue;
            }
            
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                
                match app.state.clone() {
                    AppState::Home => {
                        handle_home_input(app, key.code)?;
                        if app.state == AppState::Scanning {
                            run_scan(terminal, app).await?;
                        }
                    }
                    AppState::PathInput => {
                        handle_path_input(app, key.code);
                    }
                    AppState::Scanning => {
                        if key.code == KeyCode::Char('q') {
                            app.state = AppState::Home;
                        }
                    }
                    AppState::ScanDetails => {
                        if handle_scan_details_input(app, key.code)? {
                            return Ok(());
                        }
                    }
                    AppState::SystemWarning => {
                        handle_system_warning_input(app, key.code);
                    }
                    AppState::Confirmation => {
                        handle_confirmation_input(app, key.code);
                    }
                    AppState::AllFiles => {
                        if handle_all_files_input(app, key.code)? {
                            return Ok(());
                        }
                    }
                    AppState::Search => {
                        handle_search_input(app, key.code);
                    }
                    _ => {
                        if handle_viewing_input(app, key.code)? {
                            return Ok(());
                        }
                    }
                }
            }
        }
        
        tokio::task::yield_now().await;
    }
}

async fn handle_mouse_event(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mouse_event: crossterm::event::MouseEvent
) -> Result<()> {
    match process_mouse_event(mouse_event) {
        MouseResult::Click(_x, _y) => {}
        MouseResult::RightClick(_x, _y) => {}
        MouseResult::ScrollUp => {
            match app.state {
                AppState::Home => {
                    if app.home_menu.selected_option > 0 {
                        app.home_menu.selected_option -= 1;
                    }
                }
                AppState::Viewing => {
                    let current = app.list_state.selected().unwrap_or(0);
                    if current > 0 {
                        app.list_state.select(Some(current - 1));
                    }
                }
                AppState::AllFiles => {
                    let current = app.all_files_state.list_state.selected().unwrap_or(0);
                    if current > 0 {
                        app.all_files_state.list_state.select(Some(current - 1));
                    }
                }
                AppState::Scanning => {
                    app.scan_scroll_offset = app.scan_scroll_offset.saturating_sub(1);
                }
                _ => {}
            }
        }
        MouseResult::ScrollDown => {
            match app.state {
                AppState::Home => {
                    if app.home_menu.selected_option < app.home_menu.options.len() - 1 {
                        app.home_menu.selected_option += 1;
                    }
                }
                AppState::Viewing => {
                    if let Some(result) = &app.scan_result {
                        let current = app.list_state.selected().unwrap_or(0);
                        if current < result.entries.len().saturating_sub(1) {
                            app.list_state.select(Some(current + 1));
                        }
                    }
                }
                AppState::AllFiles => {
                    if let Some(result) = &app.scan_result {
                        let current = app.all_files_state.list_state.selected().unwrap_or(0);
                        if current < result.entries.len().saturating_sub(1) {
                            app.all_files_state.list_state.select(Some(current + 1));
                        }
                    }
                }
                AppState::Scanning => {
                    let max_scroll = app.last_progress_snapshot.top_entries.len().saturating_sub(5);
                    app.scan_scroll_offset = (app.scan_scroll_offset + 1).min(max_scroll);
                }
                _ => {}
            }
        }
        MouseResult::DoubleClick(_x, _y) => {
            if app.state == AppState::Home {
                let selected = &app.home_menu.options[app.home_menu.selected_option];
                if matches!(selected, ScanOption::CustomPath) && app.home_menu.custom_path.is_empty() {
                    app.state = AppState::PathInput;
                    app.path_input.clear();
                    app.update_path_suggestions();
                } else {
                    app.scan_path = app.get_scan_path_from_option();
                    app.current_path = app.scan_path.clone();
                    app.state = AppState::Scanning;
                    run_scan(terminal, app).await?;
                }
            }
        }
        MouseResult::None => {}
    }
    Ok(())
}

fn handle_home_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') => std::process::exit(0),
        KeyCode::Up | KeyCode::Char('k') => {
            if app.home_menu.selected_option > 0 {
                app.home_menu.selected_option -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.home_menu.selected_option < app.home_menu.options.len() - 1 {
                app.home_menu.selected_option += 1;
            }
        }
        KeyCode::Enter => {
            let selected = &app.home_menu.options[app.home_menu.selected_option];
            if matches!(selected, ScanOption::CustomPath) && app.home_menu.custom_path.is_empty() {
                app.state = AppState::PathInput;
                app.path_input.clear();
                app.update_path_suggestions();
            } else {
                app.scan_path = app.get_scan_path_from_option();
                app.current_path = app.scan_path.clone();
                app.state = AppState::Scanning;
            }
        }
        KeyCode::Char('p') => {
            app.state = AppState::PathInput;
            app.path_input = app.home_menu.custom_path.clone();
            app.path_cursor = app.path_input.len();
            app.update_path_suggestions();
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.home_menu.min_size_mb = (app.home_menu.min_size_mb + 1).min(1000);
        }
        KeyCode::Char('-') => {
            app.home_menu.min_size_mb = app.home_menu.min_size_mb.saturating_sub(1).max(1);
        }
        KeyCode::Char('d') => {
            app.home_menu.max_depth = if app.home_menu.max_depth == 0 { 5 } else { (app.home_menu.max_depth + 1) % 11 };
        }
        KeyCode::Char('.') => {
            app.home_menu.include_hidden = !app.home_menu.include_hidden;
        }
        _ => {}
    }
    Ok(())
}

fn handle_path_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.state = AppState::Home;
        }
        KeyCode::Enter => {
            if !app.path_input.is_empty() && PathBuf::from(&app.path_input).exists() {
                app.home_menu.custom_path = app.path_input.clone();
                app.home_menu.selected_option = 2;
            }
            app.state = AppState::Home;
        }
        KeyCode::Tab => {
            if !app.home_menu.path_suggestions.is_empty() {
                app.path_input = app.home_menu.path_suggestions[0].clone();
                app.path_cursor = app.path_input.len();
            }
            app.update_path_suggestions();
        }
        KeyCode::Backspace => {
            if app.path_cursor > 0 {
                app.path_input.remove(app.path_cursor - 1);
                app.path_cursor -= 1;
            }
            app.update_path_suggestions();
        }
        KeyCode::Delete => {
            if app.path_cursor < app.path_input.len() {
                app.path_input.remove(app.path_cursor);
            }
            app.update_path_suggestions();
        }
        KeyCode::Left => {
            app.path_cursor = app.path_cursor.saturating_sub(1);
        }
        KeyCode::Right => {
            app.path_cursor = (app.path_cursor + 1).min(app.path_input.len());
        }
        KeyCode::Char(c) => {
            app.path_input.insert(app.path_cursor, c);
            app.path_cursor += 1;
            app.update_path_suggestions();
        }
        _ => {}
    }
}

fn handle_system_warning_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            for idx in &app.pending_system_deletions {
                if let Some(pos) = app.marked_for_deletion.iter().position(|&x| x == *idx) {
                    app.marked_for_deletion.remove(pos);
                }
            }
            app.pending_system_deletions.clear();
            app.status_message = "System files unmarked".to_string();
            if !app.marked_for_deletion.is_empty() {
                app.delete_marked();
            } else {
                app.state = AppState::Viewing;
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            for idx in &app.pending_system_deletions {
                if let Some(pos) = app.marked_for_deletion.iter().position(|&x| x == *idx) {
                    app.marked_for_deletion.remove(pos);
                }
            }
            app.pending_system_deletions.clear();
            app.state = AppState::Viewing;
            app.status_message = "Cancelled".to_string();
        }
        _ => {}
    }
}

fn handle_confirmation_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_deletion();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.state = AppState::Viewing;
            app.status_message = "Cancelled".to_string();
        }
        _ => {}
    }
}

fn handle_scan_details_input(app: &mut App, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Enter | KeyCode::Char('b') => {
            app.state = AppState::Viewing;
            app.status_message = "Browse files · Space to select · d to delete".to_string();
        }
        KeyCode::Char('c') => {
            app.current_view = ViewMode::Categories;
            app.state = AppState::Viewing;
        }
        KeyCode::Char('s') => {
            if let Some(ref result) = app.scan_result {
                app.marked_for_deletion = result.entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        let cat = Analyzer::categorize_file(e);
                        cat.is_safe_to_delete() && !e.is_system
                    })
                    .map(|(i, _)| i)
                    .collect();
                let size: u64 = app.marked_for_deletion.iter()
                    .filter_map(|&i| result.entries.get(i))
                    .map(|e| e.size)
                    .sum();
                app.status_message = format!(
                    "✓ {} safe items selected · {}",
                    app.marked_for_deletion.len(),
                    humansize::format_size(size, humansize::DECIMAL)
                );
            }
            app.state = AppState::Viewing;
        }
        KeyCode::Esc => {
            app.state = AppState::Viewing;
        }
        KeyCode::Char('h') => {
            app.state = AppState::Home;
            app.scan_result = None;
            app.marked_for_deletion.clear();
            app.navigation_stack.clear();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_viewing_input(app: &mut App, key: KeyCode) -> Result<bool> {
    if app.browse_search_active {
        match key {
            KeyCode::Esc => {
                app.browse_search_active = false;
                app.browse_search_query.clear();
                app.status_message = "Search cancelled".to_string();
            }
            KeyCode::Enter => {
                app.browse_search_active = false;
                if app.browse_search_query.is_empty() {
                    app.status_message = "Showing all files".to_string();
                } else {
                    let count = app.get_current_entries().len();
                    app.status_message = format!("Found {} items matching \"{}\"", count, app.browse_search_query);
                }
                app.list_state.select(Some(0));
            }
            KeyCode::Backspace => {
                app.browse_search_query.pop();
            }
            KeyCode::Char(c) => {
                app.browse_search_query.push(c);
            }
            _ => {}
        }
        return Ok(false);
    }
    
    match key {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Down | KeyCode::Char('j') => app.next_item(),
        KeyCode::Up | KeyCode::Char('k') => app.previous_item(),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
            if app.current_view == ViewMode::AllFiles {
                app.enter_folder();
            } else if app.current_view == ViewMode::Categories {
                app.enter_category_view();
            }
        }
        KeyCode::Left | KeyCode::Backspace => {
            if app.state == AppState::CategoryView {
                app.state = AppState::Viewing;
                app.selected_category = None;
                app.status_message = "Back".to_string();
            } else {
                app.go_back();
            }
        }
        KeyCode::Char(' ') => app.toggle_mark(),
        KeyCode::Char('d') => app.delete_marked(),
        KeyCode::Char('?') => app.show_help = !app.show_help,
        KeyCode::Char('v') => app.switch_view(),
        KeyCode::Char('.') => app.toggle_hidden(),
        KeyCode::Char('/') => {
            app.browse_search_active = true;
            app.browse_search_query.clear();
            app.status_message = "Type to search... (Enter to confirm, Esc to cancel)".to_string();
        }
        KeyCode::Char('o') => {
            app.browse_sort_mode = app.browse_sort_mode.cycle();
            app.list_state.select(Some(0));
            app.status_message = format!("Sort: {}", app.browse_sort_mode.name());
        }
        KeyCode::Char('a') => {
            if app.current_view == ViewMode::AllFiles {
                let current_entries = app.get_current_entries();
                let indices_to_add: Vec<usize> = current_entries
                    .iter()
                    .filter(|(actual_idx, entry)| {
                        !entry.is_system && !app.marked_for_deletion.contains(actual_idx)
                    })
                    .map(|(actual_idx, _)| *actual_idx)
                    .collect();
                app.marked_for_deletion.extend(indices_to_add);
                app.status_message = format!("{} items marked", app.marked_for_deletion.len());
            }
        }
        KeyCode::Char('s') => {
            if let Some(ref result) = app.scan_result {
                app.marked_for_deletion = result.entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        let cat = Analyzer::categorize_file(e);
                        cat.is_safe_to_delete() && !e.is_system
                    })
                    .map(|(i, _)| i)
                    .collect();
                let size: u64 = app.marked_for_deletion.iter()
                    .filter_map(|&i| result.entries.get(i))
                    .map(|e| e.size)
                    .sum();
                app.status_message = format!(
                    "✓ {} safe items · {}",
                    app.marked_for_deletion.len(),
                    humansize::format_size(size, humansize::DECIMAL)
                );
            }
        }
        KeyCode::Char('c') => {
            app.marked_for_deletion.clear();
            app.browse_search_query.clear();
            app.status_message = "Selection and search cleared".to_string();
        }
        KeyCode::Char('h') | KeyCode::Char('H') => {
            app.state = AppState::Home;
            app.scan_result = None;
            app.marked_for_deletion.clear();
            app.navigation_stack.clear();
            app.browse_search_query.clear();
        }
        KeyCode::Char('F') => {
            app.state = AppState::AllFiles;
            app.all_files_state.list_state.select(Some(0));
        }
        KeyCode::Char('i') => {
            app.state = AppState::ScanDetails;
        }
        KeyCode::Char('b') => {
            app.state = AppState::Viewing;
            app.current_view = ViewMode::AllFiles;
            app.list_state.select(Some(0));
            app.status_message = "Browse files · Space to select · d to delete".to_string();
        }
        KeyCode::Esc => {
            if app.show_help {
                app.show_help = false;
            } else if !app.browse_search_query.is_empty() {
                app.browse_search_query.clear();
                app.list_state.select(Some(0));
                app.status_message = "Search cleared".to_string();
            } else if app.state == AppState::CategoryView {
                app.state = AppState::Viewing;
                app.selected_category = None;
            } else if !app.navigation_stack.is_empty() {
                app.go_back();
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_all_files_input(app: &mut App, key: KeyCode) -> Result<bool> {
    use super::super::screens::all_files::get_filtered_entries;
    
    if app.all_files_state.search_active {
        match key {
            KeyCode::Esc => {
                app.all_files_state.search_active = false;
            }
            KeyCode::Enter => {
                app.all_files_state.search_active = false;
            }
            KeyCode::Backspace => {
                app.all_files_state.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.all_files_state.search_query.push(c);
            }
            _ => {}
        }
        return Ok(false);
    }
    
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.state = AppState::Viewing;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(result) = &app.scan_result {
                let filtered_entries = get_filtered_entries(&result.entries, &app.all_files_state, &app.marked_for_deletion);
                let current = app.all_files_state.list_state.selected().unwrap_or(0);
                if current < filtered_entries.len().saturating_sub(1) {
                    app.all_files_state.list_state.select(Some(current + 1));
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let current = app.all_files_state.list_state.selected().unwrap_or(0);
            if current > 0 {
                app.all_files_state.list_state.select(Some(current - 1));
            }
        }
        KeyCode::PageDown => {
            if let Some(result) = &app.scan_result {
                let filtered_entries = get_filtered_entries(&result.entries, &app.all_files_state, &app.marked_for_deletion);
                let current = app.all_files_state.list_state.selected().unwrap_or(0);
                let new_idx = (current + 10).min(filtered_entries.len().saturating_sub(1));
                app.all_files_state.list_state.select(Some(new_idx));
            }
        }
        KeyCode::PageUp => {
            let current = app.all_files_state.list_state.selected().unwrap_or(0);
            let new_idx = current.saturating_sub(10);
            app.all_files_state.list_state.select(Some(new_idx));
        }
        KeyCode::Home => {
            app.all_files_state.list_state.select(Some(0));
        }
        KeyCode::End => {
            if let Some(result) = &app.scan_result {
                let filtered_entries = get_filtered_entries(&result.entries, &app.all_files_state, &app.marked_for_deletion);
                app.all_files_state.list_state.select(Some(filtered_entries.len().saturating_sub(1)));
            }
        }
        KeyCode::Char(' ') => {
            if let Some(result) = &app.scan_result {
                let filtered_entries = get_filtered_entries(&result.entries, &app.all_files_state, &app.marked_for_deletion);
                if let Some(selected) = app.all_files_state.list_state.selected() {
                    if let Some((original_idx, _)) = filtered_entries.get(selected) {
                        let original_idx = *original_idx;
                        if app.marked_for_deletion.contains(&original_idx) {
                            app.marked_for_deletion.retain(|&x| x != original_idx);
                        } else {
                            app.marked_for_deletion.push(original_idx);
                        }
                    }
                }
            }
        }
        KeyCode::Char('d') => {
            if !app.marked_for_deletion.is_empty() {
                app.delete_marked();
            }
        }
        KeyCode::Char('s') | KeyCode::Char('o') => {
            app.all_files_state.cycle_sort();
            app.all_files_state.list_state.select(Some(0));
        }
        KeyCode::Char('t') => {
            app.all_files_state.cycle_filter();
            app.all_files_state.list_state.select(Some(0));
        }
        KeyCode::Char('/') => {
            app.all_files_state.search_active = true;
            app.all_files_state.search_query.clear();
        }
        KeyCode::Char('a') => {
            if let Some(result) = &app.scan_result {
                let filtered_entries = get_filtered_entries(&result.entries, &app.all_files_state, &app.marked_for_deletion);
                for (original_idx, entry) in filtered_entries {
                    if !entry.is_system && !app.marked_for_deletion.contains(&original_idx) {
                        app.marked_for_deletion.push(original_idx);
                    }
                }
            }
        }
        KeyCode::Char('c') => {
            app.marked_for_deletion.clear();
        }
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
        }
        _ => {}
    }
    Ok(false)
}

fn handle_search_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.state = AppState::AllFiles;
            app.all_files_state.search_active = false;
        }
        KeyCode::Enter => {
            app.state = AppState::AllFiles;
            app.all_files_state.search_active = false;
        }
        KeyCode::Backspace => {
            app.all_files_state.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.all_files_state.search_query.push(c);
        }
        _ => {}
    }
}

fn render_ui(f: &mut Frame, app: &mut App) {
    f.render_widget(Clear, f.area());
    
    match app.state {
        AppState::Home => {
            render_home(f, &app.home_menu, app.frame_count);
        }
        AppState::PathInput => {
            render_home(f, &app.home_menu, app.frame_count);
            render_path_input(f, &app.path_input, app.path_cursor, &app.home_menu.path_suggestions);
        }
        AppState::Scanning => {
            render_scanning_enhanced(f, app, app.frame_count, app.scan_scroll_offset);
        }
        AppState::ScanDetails => {
            render_scan_details(f, app, f.area());
        }
        AppState::SystemWarning => {
            render_results_view(f, app, f.area());
            render_system_warning_dialog(f, &app.system_warning_message, f.area());
        }
        AppState::Confirmation => {
            render_results_view(f, app, f.area());
            render_confirmation_dialog(f, &app.status_message, f.area());
        }
        AppState::AllFiles | AppState::Search => {
            super::super::screens::all_files::render_all_files_screen(f, app, f.area());
            if app.show_help {
                render_help_overlay(f, f.area());
            }
        }
        _ => {
            render_results_view(f, app, f.area());
            if app.show_help {
                render_help_overlay(f, f.area());
            }
        }
    }
}
