use std::path::PathBuf;

use clap::{Parser, Subcommand};



#[derive(Parser)]
#[command(name = "yari-rs")]
#[command(author = "fiji <me@fiji-flo.de>")]
#[command(version = "1.0")]
#[command(about = "Rusty Yari", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
        #[arg(long, default_value_t = false)]
        html: bool,
    }
}

fn main() {
    println!("hello tests");
}