use crate::models::FileEntry;
use super::state::{App, BrowseSortMode};

impl App {
    /// Calculate the total size of a folder by summing all its descendants
    pub fn calculate_folder_size(&self, folder_path: &std::path::Path) -> u64 {
        if let Some(ref result) = self.scan_result {
            result.entries
                .iter()
                .filter(|e| e.path.starts_with(folder_path) && e.path != folder_path)
                .map(|e| e.size)
                .sum()
        } else {
            0
        }
    }

    pub fn get_current_entries(&self) -> Vec<(usize, &FileEntry)> {
        if let Some(ref result) = self.scan_result {
            let mut entries: Vec<(usize, &FileEntry)> = result.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    if let Some(parent) = e.path.parent() {
                        parent == self.current_path
                    } else {
                        false
                    }
                })
                .filter(|(_, e)| self.show_hidden || !e.is_hidden)
                .filter(|(_, e)| {
                    if self.browse_search_query.is_empty() {
                        true
                    } else {
                        e.name.to_lowercase().contains(&self.browse_search_query.to_lowercase())
                    }
                })
                .collect();
            
            match self.browse_sort_mode {
                BrowseSortMode::SizeDesc => {
                    entries.sort_by(|a, b| {
                        let size_a = if a.1.is_dir {
                            self.calculate_folder_size(&a.1.path)
                        } else {
                            a.1.size
                        };
                        let size_b = if b.1.is_dir {
                            self.calculate_folder_size(&b.1.path)
                        } else {
                            b.1.size
                        };
                        size_b.cmp(&size_a)
                    });
                }
                BrowseSortMode::SizeAsc => {
                    entries.sort_by(|a, b| {
                        let size_a = if a.1.is_dir {
                            self.calculate_folder_size(&a.1.path)
                        } else {
                            a.1.size
                        };
                        let size_b = if b.1.is_dir {
                            self.calculate_folder_size(&b.1.path)
                        } else {
                            b.1.size
                        };
                        size_a.cmp(&size_b)
                    });
                }
                BrowseSortMode::NameAsc => {
                    entries.sort_by(|a, b| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()));
                }
                BrowseSortMode::NameDesc => {
                    entries.sort_by(|a, b| b.1.name.to_lowercase().cmp(&a.1.name.to_lowercase()));
                }
                BrowseSortMode::DateDesc => {
                    entries.sort_by(|a, b| b.1.modified.cmp(&a.1.modified));
                }
                BrowseSortMode::DateAsc => {
                    entries.sort_by(|a, b| a.1.modified.cmp(&b.1.modified));
                }
            }
            
            entries
        } else {
            Vec::new()
        }
    }
    
    pub fn get_entry_display_size(&self, entry: &FileEntry) -> u64 {
        if entry.is_dir {
            self.calculate_folder_size(&entry.path)
        } else {
            entry.size
        }
    }

    #[allow(dead_code)]
    pub fn get_visible_entries(&self) -> Vec<(usize, &FileEntry)> {
        if let Some(ref result) = self.scan_result {
            result.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| self.show_hidden || !e.is_hidden)
                .collect()
        } else {
            Vec::new()
        }
    }
}
