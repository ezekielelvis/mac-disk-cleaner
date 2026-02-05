// App Module - Main application state and logic
// Split into submodules for better organization

mod state;
mod navigation;
mod deletion;
mod entries;
mod scan;
mod run;

pub use state::*;
pub use run::run_app;
