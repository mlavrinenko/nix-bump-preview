use anyhow::Result;
use clap::Parser;

/// Preview a Nix flake input bump: package version deltas and build cost, without building or switching
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Name to greet
    #[arg(default_value = "nix-bump-preview")]
    name: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    nix_bump_preview::greet(&cli.name)?;

    Ok(())
}
