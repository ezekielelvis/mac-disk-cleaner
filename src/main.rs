mod models;
mod scanner;
mod web;
mod analyzer;
mod cleaner;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "disk-cleaner")]
#[command(about = "Smart disk space analyzer and cleaner (web UI)", long_about = None)]
struct Args {
    /// Default directory to pre-fill in the UI (defaults to root / for full disk)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Minimum file size to display (in MB)
    #[arg(short, long, default_value = "1")]
    min_size: u64,

    /// Scan depth limit (0 = unlimited)
    #[arg(short, long, default_value = "0")]
    depth: usize,

    /// Use home directory as the default scan path instead of full disk
    #[arg(long)]
    home: bool,

    /// Port for the web UI
    #[arg(long, default_value = "8080")]
    port: u16,

    /// Do not open the browser automatically
    #[arg(long)]
    no_open: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let default_path = if let Some(path) = args.path {
        path
    } else if args.home {
        dirs::home_dir().expect("Failed to get home directory")
    } else {
        PathBuf::from("/")
    };

    if !args.no_open {
        let url = format!("http://127.0.0.1:{}", args.port);
        tokio::spawn(async move {
            // Give the server a moment to bind, then best-effort open the browser.
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            let _ = open_browser(&url);
        });
    }

    web::run_server(default_path, args.min_size, args.depth, args.port).await?;

    Ok(())
}

fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";

    std::process::Command::new(cmd).arg(url).spawn().map(|_| ())
}
