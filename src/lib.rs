pub mod builder;
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod installer;
pub mod package;
pub mod util;
pub mod vcs;

pub use error::{IjiError, Result};

use crate::{
    cli::Cli,
    config::Config,
    util::log::{self, Verbosity},
};

pub fn run(cli: Cli) -> Result<()> {
    log::set_verbosity(match (cli.quiet, cli.verbose) {
        (true, _) => Verbosity::Quiet,
        (_, true) => Verbosity::Verbose,
        _ => Verbosity::Normal,
    });
    let cfg = Config::load()?;
    commands::dispatch(&cfg, cli.command)
}
