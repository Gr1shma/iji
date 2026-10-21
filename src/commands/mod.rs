use crate::Result;
use crate::cli::Commands;
use crate::config::Config;

mod build;
pub mod install;
pub mod list;
pub mod remove;
pub mod update;

pub fn dispatch(cfg: &Config, command: Commands) -> Result<()> {
    match command {
        Commands::Install { packages, force } => install::run(cfg, &packages, force),
        Commands::List => list::run(cfg),
        Commands::Update { packages } => update::run(cfg, &packages),
        Commands::Remove { packages } => remove::run(cfg, &packages),
    }
}
