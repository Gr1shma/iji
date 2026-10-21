use crate::cli::Commands;
use crate::config::Config;
use crate::{IjiError, Result};

pub mod install;

pub fn dispatch(cfg: &Config, command: Commands) -> Result<()> {
    match command {
        Commands::Install { packages, force } => install::run(cfg, &packages, force),
        Commands::List => Err(IjiError::NotImplemented("list")),
        Commands::Update { .. } => Err(IjiError::NotImplemented("update")),
        Commands::Remove { .. } => Err(IjiError::NotImplemented("remove")),
    }
}
