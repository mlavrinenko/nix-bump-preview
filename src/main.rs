use std::process::ExitCode;

use clap::Parser;
use nix_bump_preview::cli::Cli;

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();
    match nix_bump_preview::run(&cli) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}
