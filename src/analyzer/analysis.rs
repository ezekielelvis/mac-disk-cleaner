use crate::models::FileEntry;
use super::category::FileCategory;
use super::rules::ClassificationRules;

pub struct Analyzer;

impl Analyzer {
    /// Categorize an owned batch of entries in O(n), preserving duplicate-name
    /// detection by computing the name-frequency map a single time.
    pub fn categorize_all(entries: Vec<FileEntry>) -> Vec<(FileEntry, FileCategory)> {
        let name_counts = ClassificationRules::build_name_counts(&entries);
        entries
            .into_iter()
            .map(|entry| {
                let category = ClassificationRules::classify_with_name_counts(&entry, &name_counts);
                (entry, category)
            })
            .collect()
    }
}
