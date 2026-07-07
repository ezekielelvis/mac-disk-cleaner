#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Cache,
    TempFiles,
    LargeFiles,
    OldFiles,
    DuplicateName,
    LogFiles,
    BuildArtifacts,
    NodeModules,
    PackageCache,
    HiddenFiles,
    SystemFiles,
    LibraryFiles,
    Downloads,
    Documents,
    Media,
    Archives,
    Regular,
}

impl FileCategory {
    pub fn as_str(&self) -> &str {
        match self {
            FileCategory::Cache => "🗑️ Cache",
            FileCategory::TempFiles => "🌡️ Temp Files",
            FileCategory::LargeFiles => "📦 Large Files",
            FileCategory::OldFiles => "📅 Old Files",
            FileCategory::DuplicateName => "👯 Duplicate Names",
            FileCategory::LogFiles => "📜 Log Files",
            FileCategory::BuildArtifacts => "🔨 Build Artifacts",
            FileCategory::NodeModules => "📦 node_modules",
            FileCategory::PackageCache => "📥 Package Cache",
            FileCategory::HiddenFiles => "👁️ Hidden Files",
            FileCategory::SystemFiles => "⚙️ System Files",
            FileCategory::LibraryFiles => "📚 Library Files",
            FileCategory::Downloads => "⬇️ Downloads",
            FileCategory::Documents => "📄 Documents",
            FileCategory::Media => "🎬 Media",
            FileCategory::Archives => "🗜️ Archives",
            FileCategory::Regular => "📁 Regular",
        }
    }

    /// CSS hex color used by the web UI for this category.
    pub fn color(&self) -> &str {
        match self {
            FileCategory::Cache => "#e5c07b",
            FileCategory::TempFiles => "#e06c75",
            FileCategory::LargeFiles => "#c678dd",
            FileCategory::OldFiles => "#56b6c2",
            FileCategory::DuplicateName => "#61afef",
            FileCategory::LogFiles => "#d19a66",
            FileCategory::BuildArtifacts => "#ff7b72",
            FileCategory::NodeModules => "#d291e4",
            FileCategory::PackageCache => "#7fdbca",
            FileCategory::HiddenFiles => "#8b95a5",
            FileCategory::SystemFiles => "#f14c4c",
            FileCategory::LibraryFiles => "#7aa2f7",
            FileCategory::Downloads => "#98c379",
            FileCategory::Documents => "#abb2bf",
            FileCategory::Media => "#89d185",
            FileCategory::Archives => "#e5c07b",
            FileCategory::Regular => "#abb2bf",
        }
    }

    pub fn is_safe_to_delete(&self) -> bool {
        match self {
            FileCategory::Cache => true,
            FileCategory::TempFiles => true,
            FileCategory::BuildArtifacts => true,
            FileCategory::NodeModules => true,
            FileCategory::PackageCache => true,
            FileCategory::LogFiles => true,
            FileCategory::OldFiles => true,
            FileCategory::Archives => true,
            FileCategory::Downloads => true,
            FileCategory::DuplicateName => true,
            FileCategory::LargeFiles => false, // Need review
            FileCategory::HiddenFiles => false, // Might be config
            FileCategory::SystemFiles => false, // Dangerous
            FileCategory::LibraryFiles => false, // App data
            FileCategory::Documents => false, // User data
            FileCategory::Media => false, // User data
            FileCategory::Regular => false,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            FileCategory::Cache => "Temporary cache files that can be safely deleted. Will be regenerated.",
            FileCategory::TempFiles => "Temporary files. Usually safe to delete.",
            FileCategory::LargeFiles => "Large files over 100MB. Review before deleting.",
            FileCategory::OldFiles => "Files not accessed in over a year. May be obsolete.",
            FileCategory::DuplicateName => "Files with same name in different locations. May be duplicates.",
            FileCategory::LogFiles => "Application log files. Can grow large over time.",
            FileCategory::BuildArtifacts => "Compiled code and build outputs. Can be regenerated.",
            FileCategory::NodeModules => "JavaScript dependencies. Can be reinstalled with npm/yarn.",
            FileCategory::PackageCache => "Downloaded packages. Can be re-downloaded when needed.",
            FileCategory::HiddenFiles => "Hidden files (start with .). May contain important configs.",
            FileCategory::SystemFiles => "⚠️ SYSTEM FILES - Required for OS operation. DO NOT DELETE!",
            FileCategory::LibraryFiles => "Application data in Library folder. Deleting may break apps.",
            FileCategory::Downloads => "Downloaded files. Review before deleting.",
            FileCategory::Documents => "User documents. Be careful!",
            FileCategory::Media => "Photos, videos, audio files. User content.",
            FileCategory::Archives => "Compressed archives (.zip, .tar, etc). May be backup copies.",
            FileCategory::Regular => "Regular files.",
        }
    }
}
