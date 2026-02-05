// Scan - Scan execution logic

use crate::analyzer::Analyzer;
use crate::scanner::Scanner;
use super::state::App;
use super::super::types::{AppState, ScanProgressSnapshot};
use super::super::screens::render_scanning_enhanced;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::CrosstermBackend, widgets::Clear, Terminal};
use std::io;

pub async fn run_scan(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let progress = app.scan_progress.clone();
    let min_size = app.home_menu.min_size_mb;
    let depth = app.home_menu.max_depth;
    
    let scanner = Scanner::new(min_size, depth);
    let scan_path_clone = app.scan_path.clone();
    let progress_clone = progress.clone();
    
    eprintln!("🔍 Starting scan of: {}", app.scan_path.display());
    
    let scan_handle = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(scanner.scan_with_progress(&scan_path_clone, progress_clone))
    });

    let mut _last_update = std::time::Instant::now();
    let mut last_files_count = 0;
    
    loop {
        app.frame_count = app.frame_count.wrapping_add(1);
        
        if app.frame_count % 3 == 0 {
            if let Ok(prog) = app.scan_progress.try_lock() {
                app.last_progress_snapshot = ScanProgressSnapshot {
                    current_path: prog.current_path.clone(),
                    files_scanned: prog.files_scanned,
                    dirs_scanned: prog.dirs_scanned,
                    total_size_scanned: prog.total_size_scanned,
                    entries_count: prog.entries.len(),
                    top_entries: prog.entries.iter()
                        .rev()
                        .take(50)
                        .map(|e| (e.name.clone(), e.size, Analyzer::categorize_file(e).as_str().to_string()))
                        .collect(),
                    category_sizes: prog.category_sizes.clone(),
                };
                
                if prog.is_complete {
                    break;
                }
                
                let current_files = prog.files_scanned;
                if current_files > last_files_count {
                    _last_update = std::time::Instant::now();
                    last_files_count = current_files;
                }
            }
        }

        terminal.draw(|f| {
            f.render_widget(Clear, f.area());
            render_scanning_enhanced(f, app, app.frame_count, app.scan_scroll_offset);
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.state = AppState::Home;
                            return Ok(());
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.scan_scroll_offset = app.scan_scroll_offset.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max_scroll = app.last_progress_snapshot.top_entries.len().saturating_sub(5);
                            app.scan_scroll_offset = (app.scan_scroll_offset + 1).min(max_scroll);
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if scan_handle.is_finished() {
            break;
        }
        
        tokio::task::yield_now().await;
    }

    match scan_handle.await {
        Ok(Ok(result)) => {
            eprintln!("✓ Scan successful: {} files, {} dirs",
                result.total_files, result.total_dirs);
            
            app.recommendations = Analyzer::get_recommendations(&result.entries);
            app.categories = Analyzer::group_by_category(&result.entries);
            
            let safe_savings = Analyzer::calculate_safe_savings(&result.entries);
            app.status_message = format!(
                "Scan complete · {} potential savings",
                humansize::format_size(safe_savings, humansize::DECIMAL)
            );
            
            app.scan_result = Some(result);
            app.state = AppState::ScanComplete;  // Go to summary first so users can choose browse
            app.list_state.select(Some(0));
            app.category_state.select(Some(0));
        }
        Ok(Err(e)) => {
            app.status_message = format!("Scan failed: {}", e);
            app.state = AppState::Home;
        }
        Err(e) => {
            app.status_message = format!("Scan error: {}", e);
            app.state = AppState::Home;
        }
    }

    Ok(())
}
