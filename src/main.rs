#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]

use ani_tui::config::Config;
use ani_tui::ui::App;
use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "ani-tui")]
#[command(about = "A Netflix-inspired TUI for anime streaming")]
#[command(version)]
struct Cli {
    /// Search query to start with
    #[arg(short, long)]
    query: Option<String>,

    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging if debug mode
    if cli.debug {
        tracing_subscriber::fmt::init();
    }

    // Load configuration
    let config = Config::load(cli.config.as_deref()).await?;

    // Initialize database
    let db = Arc::new(ani_tui::db::Database::new().await?);

    // Run TUI application
    let mut app = App::new(config, db).await?;

    if let Some(query) = cli.query {
        app.set_initial_search(query);
    }

    app.run().await?;

    Ok(())
}
