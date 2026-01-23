mod scanner;
mod ui;
mod analyzer;
mod cleaner;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "disk-cleaner")]
#[command(about = "Smart disk space analyzer and cleaner", long_about = None)]
struct Args {
    /// Directory to scan (defaults to home directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Minimum file size to display (in MB)
    #[arg(short, long, default_value = "1")]
    min_size: u64,

    /// Scan depth limit
    #[arg(short, long, default_value = "10")]
    depth: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let scan_path = args.path.unwrap_or_else(|| {
        dirs::home_dir().expect("Failed to get home directory")
    });

    // Run the TUI application
    ui::run_app(scan_path, args.min_size, args.depth).await?;
    
    Ok(())
}
