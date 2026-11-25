use std::process::ExitCode;

use clap::Parser;
use zrs::ZSH_INIT_SCRIPT;
use zrs::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("zrs: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), zrs::error::ZrsError> {
    match cli.command {
        Command::Init => {
            print!("{ZSH_INIT_SCRIPT}");
            Ok(())
        }
        Command::Validate { config } => {
            let path = match config {
                Some(path) => path,
                None => zrs::config::default_config_path()?,
            };
            let _ = zrs::config::load_from_path(&path)?;
            Ok(())
        }
    }
}
