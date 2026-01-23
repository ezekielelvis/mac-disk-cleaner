# Disk Cleaner 🧹

A powerful, smart CLI disk space analyzer and cleaner built with Rust and Ratatui. This tool helps you identify and remove unwanted files intelligently, freeing up disk space on your system.

## Features ✨

- **Full Disk Scan**: Scans the entire disk from root `/` by default with parallel workers
- **Fast Parallel Scanning**: Uses 4 worker threads for significantly faster scans
- **Smart Virtual FS Handling**: Automatically skips virtual filesystems (/dev, /proc, etc.)
- **Accurate Disk Usage**: Uses block-level allocation for true disk space calculation
- **Smart Categorization**: Automatically categorizes files into:
  - Cache files
  - Temporary files
  - Large files (>100MB)
  - Old files (>1 year)
  - Log files
  - Build artifacts (target/, build/, dist/, .next/)
  - node_modules directories
  - Package caches (.cargo, .npm, .yarn, pip)

- **Interactive TUI**: Beautiful terminal user interface built with Ratatui
- **Color-coded Display**: Different colors for different file categories
- **System File Protection**: System files marked with ⚙️ and warnings before deletion
- **Hidden File Detection**: Hidden files marked with ◌ indicator
- **Smart Recommendations**: AI-powered suggestions on what to clean
- **Batch Operations**: Mark multiple files/directories for deletion
- **Safe Deletion**: Confirmation prompts before deletion
- **Detailed Information**: View file sizes, modification dates, and paths

## Installation 🚀

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Build from Source

```bash
# Clone the repository
cd disk-cleaner2

# Build the project
cargo build --release

# Run the binary
./target/release/disk-cleaner
```

### Install

```bash
cargo install --path .
```

## Usage 📖

### Basic Usage

```bash
# Full disk scan (default - scans from /)
disk-cleaner

# Scan only home directory
disk-cleaner --home

# Scan a specific directory
disk-cleaner --path /path/to/directory

# Set minimum file size (in MB)
disk-cleaner --min-size 10

# Unlimited depth (default is 0 = unlimited)
disk-cleaner --depth 0
```

### Keyboard Shortcuts

**Navigation:**
- `↑`/`k` - Move up
- `↓`/`j` - Move down

**Actions:**
- `Space` - Mark/unmark item for deletion
- `d` - Delete marked items
- `a` - Mark all items in current view
- `c` - Clear all marks
- `v` - Switch between file list and category view

**Other:**
- `h`/`?` - Toggle help screen
- `q` - Quit application

## Features in Detail 🔍

### Smart Analysis

The tool automatically identifies:
- **node_modules**: JavaScript/TypeScript dependencies that can be reinstalled
- **Build Artifacts**: Compiled code (target/, build/, dist/) that can be regenerated
- **Cache Directories**: Various caches that can be safely cleared
- **Log Files**: Old log files taking up space
- **Large Files**: Files over 100MB that might need attention
- **Old Files**: Files not modified in over a year

### Category View

Switch to category view (press `v`) to see files grouped by type:
- Quick overview of space used by each category
- Color-coded for easy identification
- Shows count and total size per category

### Safety Features

- Confirmation required before deletion
- Shows estimated space to be freed
- Reports success/failure for each deletion
- Non-destructive marking system

## Architecture 🏗️

The project is organized into modules:

- **scanner**: File system traversal and size calculation
- **analyzer**: Smart categorization and recommendation engine
- **cleaner**: Safe file deletion operations
- **ui**: Ratatui-based terminal user interface

## Dependencies 📦

- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal manipulation
- `clap` - Command-line argument parsing
- `tokio` - Async runtime
- `walkdir` - Directory traversal
- `humansize` - Human-readable file sizes
- `chrono` - Date and time handling

## Examples 💡

### Find and clean node_modules

1. Run: `disk-cleaner --path ~/projects`
2. Press `v` to switch to category view
3. Navigate to "node_modules" category
4. Review the space usage
5. Press `a` to mark all, then `d` to delete

### Clean old build artifacts

1. Run the cleaner on your projects directory
2. Look for "Build Artifacts" in the recommendations
3. Mark individual directories or use category view
4. Confirm deletion to free up space

### Find large files

1. Run with: `disk-cleaner --min-size 100`
2. All files >100MB will be shown at the top
3. Review and mark unwanted large files
4. Delete to reclaim space

## Contributing 🤝

Contributions are welcome! Please feel free to submit a Pull Request.

## License 📄

This project is licensed under the MIT License.

## Safety Warning ⚠️

This tool permanently deletes files. Always review marked items carefully before confirming deletion. The authors are not responsible for data loss.

## Roadmap 🗺️

- [ ] Duplicate file detection
- [ ] Export scan results to JSON/CSV
- [ ] Scheduled automatic cleaning
- [ ] Cloud storage integration
- [ ] Custom rules engine
- [ ] File preview
- [ ] Undo functionality (trash/recycle bin)

---

Built with ❤️ using Rust and Ratatui
