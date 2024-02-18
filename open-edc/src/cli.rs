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
    Start {
        #[clap(short, long, help = "Url for the server")]
        url: Option<String>,
        #[clap(short, long, help = "Port the server should run on")]
        port: Option<usize>,
    },
}
