use anyhow::Result;
use std::path::Path;
use std::fs;
use walkdir::WalkDir;

pub struct Cleaner;

impl Cleaner {
    pub fn delete_file(path: &Path) -> Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn delete_files(paths: &[&Path]) -> Result<Vec<(String, bool)>> {
        let mut results = Vec::new();

        for path in paths {
            let path_str = path.to_string_lossy().to_string();
            match Self::delete_file(path) {
                Ok(_) => results.push((path_str, true)),
                Err(_) => results.push((path_str, false)),
            }
        }

        Ok(results)
    }

    pub fn estimate_space_freed(paths: &[&Path]) -> u64 {
        paths.iter()
            .map(|path| {
                if path.is_dir() {
                    WalkDir::new(path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .filter(|m| m.is_file())
                        .map(|m| m.len())
                        .sum()
                } else {
                    path.metadata().map(|m| m.len()).unwrap_or(0)
                }
            })
            .sum()
    }
}
