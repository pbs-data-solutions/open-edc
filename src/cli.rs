use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about = "CLI for the Open EDC server")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the server
    Start {},
}
