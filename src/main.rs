use std::process::ExitCode;

use clap::Parser;
use iji::{cli::Cli, util::log};

fn main() -> ExitCode {
    match iji::run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error(e);
            ExitCode::FAILURE
        }
    }
}
