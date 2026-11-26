use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "zrs", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print Zsh ZLE initialization script to stdout
    Init {
        /// Define widgets without installing default key bindings
        #[arg(long)]
        no_keybinds: bool,
    },
    /// Validate configuration YAML
    Validate {
        /// Path to config file (defaults to $XDG_CONFIG_HOME/zrs/config.yaml)
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Expand the final word using NUL-terminated Zsh tokens from stdin
    Expand(BufferArgs),
    /// Move to the next placeholder
    Placeholder(BufferArgs),
    /// Search and insert a snippet
    Snippet(BufferArgs),
    /// Select a command from Zsh history provided as NUL-terminated records on stdin
    History(BufferArgs),
    /// Select a completion for the current input
    Complete {
        #[command(flatten)]
        buffer: BufferArgs,
        /// Final Zsh token to replace (empty at a word boundary)
        #[arg(long, require_equals = true)]
        word: Option<String>,
        /// Unquoted search query for the final Zsh token
        #[arg(long, require_equals = true, requires = "word")]
        query: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct BufferArgs {
    #[arg(long, require_equals = true)]
    pub lbuffer: String,
    #[arg(long, require_equals = true)]
    pub rbuffer: String,
}
