use anyhow::Result;
use std::path::Path;
use std::fs;

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
}
