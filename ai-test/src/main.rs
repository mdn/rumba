use std::path::PathBuf;

use anyhow::Error;
use clap::{Parser, Subcommand};
use rumba::logging::init_logging;

use crate::ai_help::ai_help_all;

mod ai_help;
mod prompts;

#[derive(Parser)]
#[command(name = "yari-rs")]
#[command(author = "fiji <me@fiji-flo.de>")]
#[command(version = "1.0")]
#[command(about = "Rusty Yari", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Test {
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    init_logging(false);

    let cli = Cli::parse();
    match cli.command {
        Commands::Test { path, out } => {
            let out = out.unwrap_or_else(|| PathBuf::from("/tmp/test"));
            ai_help_all(path, out).await?;
        }
    }
    Ok(())
}
