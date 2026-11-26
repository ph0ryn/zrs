use std::io::{self, Read, Write};
use std::process::ExitCode;

use clap::Parser;
use zrs_zsh::cli::{BufferArgs, Cli, Command};
use zrs_zsh::error::ZrsError;
use zrs_zsh::protocol::Outcome;
use zrs_zsh::{ZSH_INIT_SCRIPT, abbrev, completion, config, history, snippet, template};

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = u8::from(error.use_stderr());
            let _ = error.print();
            return ExitCode::from(code);
        }
    };

    match run(cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("zrs-zsh: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<u8, ZrsError> {
    let outcome: Outcome = match command {
        Command::Init { no_keybinds } => {
            let mut out = io::stdout().lock();
            out.write_all(ZSH_INIT_SCRIPT.as_bytes())?;
            if !no_keybinds {
                writeln!(out, "_zrs_bind_defaults")?;
            }
            return Ok(0);
        }
        Command::Validate { config } => {
            let path = match config {
                Some(path) => path,
                None => config::default_config_path()?,
            };
            config::load_from_path(&path)?;
            return Ok(0);
        }
        Command::Expand(BufferArgs { lbuffer, rbuffer }) => {
            let cfg = config::load_interactive()?;
            let mut tokens = String::new();
            io::stdin().read_to_string(&mut tokens)?;
            if !tokens.is_empty() && !tokens.ends_with('\0') {
                return Err(ZrsError::Protocol("input tokens are not NUL-terminated"));
            }
            let tokens: Vec<_> = tokens.split_terminator('\0').collect();
            abbrev::expand(&cfg.abbreviations, &lbuffer, &rbuffer, &tokens)?.into()
        }
        Command::Placeholder(BufferArgs { lbuffer, rbuffer }) => {
            template::next(&lbuffer, &rbuffer).into()
        }
        Command::Snippet(BufferArgs { lbuffer, rbuffer }) => {
            snippet::select(&config::load_interactive()?.snippets, &lbuffer, &rbuffer)?
        }
        Command::History(BufferArgs { lbuffer, rbuffer }) => {
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input)?;
            history::select(&input, &lbuffer, &rbuffer)?
        }
        Command::Complete {
            buffer: BufferArgs { lbuffer, rbuffer },
            word,
            query,
        } => completion::complete(
            &config::load_interactive()?.completions,
            &lbuffer,
            &rbuffer,
            word.as_deref(),
            query.as_deref(),
        )?,
    };
    outcome.write(io::stdout().lock())
}
