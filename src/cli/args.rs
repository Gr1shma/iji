use clap::{Parser, Subcommand};

/// iji - simple package manager
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install one or more packages
    #[command(visible_alias = "i")]
    Install {
        #[arg(required = true, value_name = "PKG")]
        packages: Vec<String>,

        /// Reinstall even if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// List installed packages
    #[command(visible_alias = "l")]
    List,

    /// Update given packages, or all if none specified
    #[command(visible_alias = "u")]
    Update {
        #[arg(value_name = "PKG")]
        packages: Vec<String>,
    },

    /// Remove one or more packages
    #[command(visible_alias = "r")]
    Remove {
        #[arg(required = true, value_name = "PKG")]
        packages: Vec<String>,
    },
}
