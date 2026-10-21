use crate::cli::Commands;
use crate::config::Config;
use crate::{IjiError, Result};

pub mod install;
pub mod list;
pub mod remove;

pub fn dispatch(cfg: &Config, command: Commands) -> Result<()> {
    match command {
        Commands::Install { packages, force } => install::run(cfg, &packages, force),
        Commands::List => list::run(cfg),
        Commands::Update { .. } => Err(IjiError::NotImplemented("update")),
        Commands::Remove { packages } => remove::run(cfg, &packages),
    }
}
