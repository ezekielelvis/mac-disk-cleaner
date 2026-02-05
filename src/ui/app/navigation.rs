use super::state::App;
use super::super::types::ViewMode;
use std::path::PathBuf;

impl App {
    pub fn next_item(&mut self) {
        let items_len = match self.current_view {
            ViewMode::AllFiles => self.get_current_entries().len(),
            ViewMode::Categories => self.categories.len(),
        };

        if items_len == 0 {
            return;
        }

        let state = match self.current_view {
            ViewMode::AllFiles => &mut self.list_state,
            ViewMode::Categories => &mut self.category_state,
        };

        let i = match state.selected() {
            Some(i) => {
                if i >= items_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn previous_item(&mut self) {
        let items_len = match self.current_view {
            ViewMode::AllFiles => self.get_current_entries().len(),
            ViewMode::Categories => self.categories.len(),
        };

        if items_len == 0 {
            return;
        }

        let state = match self.current_view {
            ViewMode::AllFiles => &mut self.list_state,
            ViewMode::Categories => &mut self.category_state,
        };

        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    items_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn enter_folder(&mut self) {
        if self.current_view != ViewMode::AllFiles {
            return;
        }

        let target_path: Option<PathBuf> = {
            let current_entries = self.get_current_entries();
            if let Some(selected_idx) = self.list_state.selected() {
                if let Some((_, entry)) = current_entries.get(selected_idx) {
                    if entry.is_dir {
                        Some(entry.path.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(new_path) = target_path {
            self.navigation_stack.push(self.current_path.clone());
            self.current_path = new_path;
            self.list_state.select(Some(0));
            self.status_message = format!("📂 {}", self.current_path.to_string_lossy());
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev_path) = self.navigation_stack.pop() {
            self.current_path = prev_path;
            self.list_state.select(Some(0));
            self.status_message = format!("📂 {}", self.current_path.to_string_lossy());
        } else if self.current_path != self.scan_path {
            if let Some(parent) = self.current_path.parent() {
                if parent.starts_with(&self.scan_path) || parent == self.scan_path {
                    self.current_path = parent.to_path_buf();
                    self.list_state.select(Some(0));
                    self.status_message = format!("📂 {}", self.current_path.to_string_lossy());
                }
            }
        }
    }

    pub fn enter_category_view(&mut self) {
        use super::super::types::AppState;
        
        if let Some(i) = self.category_state.selected() {
            let mut categories: Vec<_> = self.categories.keys().collect();
            categories.sort_by_key(|c| c.as_str());
            if let Some(&category) = categories.get(i) {
                self.selected_category = Some(*category);
                self.state = AppState::CategoryView;
                self.status_message = format!("{} · {}", category.as_str(), category.description());
            }
        }
    }
}
