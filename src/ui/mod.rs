// UI Module - Organized into components, screens, and handlers
//
// Structure:
// - components/  - Reusable UI widgets (headers, footers, dialogs, etc.)
// - screens/     - Full-screen renderers (home, scanning, results, all_files)
// - handlers/    - Input event handlers (keyboard, mouse)
// - app.rs       - Main application state and logic
// - types.rs     - Type definitions
// - colors.rs    - Color constants

mod app;
pub mod colors;
pub mod types;
pub mod components;
pub mod screens;
pub mod handlers;

// Legacy modules - keeping for backwards compatibility during transition
mod render_home;
mod render_scanning;
mod render_results;

pub use app::run_app;
