use crate::analyzer::Analyzer;
use crate::cleaner::Cleaner;
use crate::scanner::get_system_warning;
use super::state::App;
use super::super::types::AppState;

impl App {
    pub fn toggle_mark(&mut self) {
        use super::super::types::ViewMode;
        
        if self.current_view != ViewMode::AllFiles {
            return;
        }

        let current_entries = self.get_current_entries();
        if let Some(visible_idx) = self.list_state.selected() {
            if let Some((actual_idx, entry)) = current_entries.get(visible_idx) {
                if entry.is_system {
                    self.status_message = "⚠️  Cannot mark system file".to_string();
                    return;
                }

                if let Some(pos) = self.marked_for_deletion.iter().position(|&x| x == *actual_idx) {
                    self.marked_for_deletion.remove(pos);
                    self.status_message = format!("Unmarked · {} selected", self.marked_for_deletion.len());
                } else {
                    self.marked_for_deletion.push(*actual_idx);
                    self.status_message = format!("Marked · {} selected", self.marked_for_deletion.len());
                }
            }
        }
    }

    pub fn delete_marked(&mut self) {
        if self.marked_for_deletion.is_empty() {
            self.status_message = "No items selected".to_string();
            return;
        }

        if let Some(ref result) = self.scan_result {
            let system_files: Vec<usize> = self.marked_for_deletion.iter()
                .filter(|&&i| result.entries.get(i).map(|e| e.is_system).unwrap_or(false))
                .cloned()
                .collect();

            if !system_files.is_empty() {
                let system_entry = result.entries.get(system_files[0]).unwrap();
                self.system_warning_message = format!(
                    "🛑 SYSTEM FILE WARNING\n\n{} system file(s) selected\n\n{}\n\n{}\n\nPress Y to proceed (dangerous) or N to cancel",
                    system_files.len(),
                    system_entry.path.to_string_lossy(),
                    get_system_warning(&system_entry.path)
                        .unwrap_or_else(|| "Critical system file".to_string())
                );
                self.pending_system_deletions = system_files;
                self.state = AppState::SystemWarning;
                return;
            }

            let paths: Vec<_> = self.marked_for_deletion.iter()
                .filter_map(|&i| result.entries.get(i))
                .map(|e| e.path.as_path())
                .collect();

            let space_to_free = Cleaner::estimate_space_freed(&paths);
            self.status_message = format!(
                "Delete {} items? Free {} · Press Y to confirm, N to cancel",
                paths.len(),
                humansize::format_size(space_to_free, humansize::DECIMAL)
            );
            self.state = AppState::Confirmation;
        }
    }

    pub fn confirm_deletion(&mut self) {
        self.state = AppState::Deleting;
        self.status_message = "Deleting...".to_string();
        
        if let Some(ref mut result) = self.scan_result {
            let to_delete: Vec<(usize, std::path::PathBuf, bool)> = self.marked_for_deletion.iter()
                .filter_map(|&i| result.entries.get(i).map(|e| (i, e)))
                .filter(|(_, e)| !e.is_system)
                .map(|(i, e)| (i, e.path.clone(), e.is_dir))
                .collect();

            let paths: Vec<_> = to_delete.iter().map(|(_, p, _)| p.as_path()).collect();
            
            if paths.is_empty() {
                self.status_message = "No deletable items".to_string();
                self.state = AppState::Viewing;
                return;
            }

            match Cleaner::delete_files(&paths) {
                Ok(results) => {
                    let success_count = results.iter().filter(|(_, success)| *success).count();
                    let failed_count = results.len() - success_count;
                    
                    let deleted_paths: Vec<std::path::PathBuf> = results.iter()
                        .zip(to_delete.iter())
                        .filter(|((_, success), _)| *success)
                        .map(|(_, (_, path, _))| path.clone())
                        .collect();
                    
                    let mut indices_to_remove: Vec<usize> = Vec::new();
                    for (idx, entry) in result.entries.iter().enumerate() {
                        for deleted_path in &deleted_paths {
                            if entry.path == *deleted_path || entry.path.starts_with(deleted_path) {
                                if !indices_to_remove.contains(&idx) {
                                    indices_to_remove.push(idx);
                                }
                                break;
                            }
                        }
                    }
                    
                    indices_to_remove.sort_by(|a, b| b.cmp(a));
                    
                    for idx in indices_to_remove {
                        if idx < result.entries.len() {
                            let removed = result.entries.remove(idx);
                            result.total_size = result.total_size.saturating_sub(removed.size);
                            if removed.is_dir {
                                result.total_dirs = result.total_dirs.saturating_sub(1);
                            } else {
                                result.total_files = result.total_files.saturating_sub(1);
                            }
                        }
                    }
                    
                    self.categories = Analyzer::group_by_category(&result.entries);
                    self.recommendations = Analyzer::get_recommendations(&result.entries);
                    self.marked_for_deletion.clear();
                    
                    use super::super::types::StorageInfo;
                    self.storage_info = StorageInfo::from_path(&self.scan_path);
                    
                    if result.entries.is_empty() {
                        self.list_state.select(None);
                    } else if let Some(selected) = self.list_state.selected() {
                        let current_entries_count = self.get_current_entries().len();
                        if selected >= current_entries_count {
                            self.list_state.select(Some(current_entries_count.saturating_sub(1)));
                        }
                    }
                    
                    if failed_count > 0 {
                        self.status_message = format!("✓ Deleted {} · ✗ {} failed", success_count, failed_count);
                    } else {
                        self.status_message = format!("✓ Deleted {} items", success_count);
                    }
                }
                Err(e) => {
                    self.status_message = format!("✗ Error: {}", e);
                }
            }
        }
        self.state = AppState::Viewing;
    }
}
