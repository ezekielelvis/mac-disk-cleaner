use anyhow::Result;
use std::path::Path;

pub struct Cleaner;

impl Cleaner {
    /// Move a file or directory to the system Trash (Recycle Bin).
    ///
    /// This is a recoverable delete: nothing is removed permanently, so a user
    /// can restore anything from the Trash if it was removed by mistake. Works
    /// for both files and directories.
    pub fn delete_file(path: &Path) -> Result<()> {
        trash::delete(path)?;
        Ok(())
    }
}
