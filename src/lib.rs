pub mod abbrev;
pub mod cli;
pub mod completion;
pub mod config;
pub mod error;
pub mod process;
pub mod protocol;
pub mod quote;
pub mod snippet;
pub mod template;

pub const ZSH_INIT_SCRIPT: &str = include_str!("../shell/zrs.zsh");
