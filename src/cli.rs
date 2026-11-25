use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "zrs", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print Zsh ZLE initialization script to stdout
    Init,
    /// Validate configuration YAML
    Validate {
        /// Path to config file (defaults to $XDG_CONFIG_HOME/zrs/config.yaml)
        #[arg(long)]
        config: Option<PathBuf>,
    },
}
