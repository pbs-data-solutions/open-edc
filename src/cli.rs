use clap::{Parser, Subcommand};
use tracing::Level;

#[derive(Debug, Parser)]
#[clap(author, version, about = "CLI for the Open EDC server")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the server
    Start {
        #[clap(short, long, help = "Set the log level")]
        log_level: Option<Level>,
    },
}
