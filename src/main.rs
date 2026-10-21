use clap::Parser;
use iji::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { packages, force } => {
            println!("pkg: {:?}, force: {:?}", packages, force)
        }
        Commands::List => println!("listing packages"),
        Commands::Update { packages } if packages.is_empty() => {
            println!("Updating all packages")
        }
        Commands::Update { packages } => println!("removing {:?}", packages),
        Commands::Remove { packages } => println!("removing {:?}", packages),
    }
}
