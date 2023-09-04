use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Error;
use clap::{Parser, Subcommand};
use rumba::experiments::Experiments;

use crate::ask::ask_all;

mod ask;
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
        #[arg(short, long)]
        experiments: Option<usize>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    println!("hello tests");
    match cli.command {
        Commands::Test {
            path,
            out,
            experiments,
        } => {
            let out = out.unwrap_or_else(|| PathBuf::from("/tmp/test"));
            let experiments = experiments.map(|i| Experiments {
                active: true,
                config: i.into(),
            });
            ask_all(path, out, experiments).await?;
        }
    }
    Ok(())
}
