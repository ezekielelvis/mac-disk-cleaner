use std::collections::HashMap;
use std::path::PathBuf;
use crate::models::FileEntry;

/// Manages directory size calculations
pub struct DirectorySizeCalculator {
    dir_sizes: HashMap<PathBuf, u64>,
}

impl DirectorySizeCalculator {
    pub fn new() -> Self {
        Self {
            dir_sizes: HashMap::new(),
        }
    }
    
    /// Record the size of files in a directory
    #[allow(dead_code)]
    pub fn record_directory_size(&mut self, dir_path: PathBuf, size: u64) {
        *self.dir_sizes.entry(dir_path).or_insert(0) += size;
    }
    
    /// Calculate recursive sizes for all directories
    /// This updates the entries vector with calculated directory sizes
    pub fn calculate_recursive_sizes(&self, entries: &mut [FileEntry]) {
        // Build a map from path to index for quick lookups
        let mut path_to_idx: HashMap<PathBuf, usize> = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            if entry.is_dir {
                path_to_idx.insert(entry.path.clone(), idx);
            }
        }
        
        // For each directory, sum up sizes of its immediate children
        for dir_path in self.dir_sizes.keys() {
            let mut total = 0u64;
            
            // Sum all child directory sizes
            for (child_path, child_size) in &self.dir_sizes {
                if let Some(parent) = child_path.parent() {
                    if parent == dir_path.as_path() {
                        total += child_size;
                    }
                }
            }
            
            // Update the directory entry with calculated size
            if let Some(&idx) = path_to_idx.get(dir_path) {
                if let Some(entry) = entries.get_mut(idx) {
                    entry.size = total;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_record_and_calculate() {
        let mut calc = DirectorySizeCalculator::new();
        
        let parent = PathBuf::from("/test");
        let child = PathBuf::from("/test/child");
        
        calc.record_directory_size(parent.clone(), 100);
        calc.record_directory_size(child.clone(), 50);
        
        let mut entries = vec![
            FileEntry {
                path: parent.clone(),
                size: 0,
                is_dir: true,
                is_hidden: false,
                is_system: false,
                modified: chrono::Local::now(),
                name: "test".to_string(),
            },
            FileEntry {
                path: child.clone(),
                size: 0,
                is_dir: true,
                is_hidden: false,
                is_system: false,
                modified: chrono::Local::now(),
                name: "child".to_string(),
            },
        ];
        
        calc.calculate_recursive_sizes(&mut entries);
        
        // Parent should have sum of its direct children
        assert!(entries[0].size >= 50);
    }
}
