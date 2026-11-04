//! Preview a Nix flake input bump without building or switching.

pub mod cli;
pub mod delta;
pub mod eval;
pub mod lock;
pub mod nix;
pub mod report;

use cli::Cli;

/// Run the preview for `cli`.
///
/// # Errors
///
/// Returns an error when the arguments are invalid, when the schema cannot be
/// rendered, or while the preview pipeline is not implemented.
pub fn run(cli: &Cli) -> anyhow::Result<()> {
    if cli.print_schema {
        println!("{}", report::schema_json()?);
        return Ok(());
    }

    cli.validate()?;
    log::debug!(
        "preview requested for {} installable(s)",
        cli.installable.len()
    );
    anyhow::bail!("preview pipeline is not implemented yet")
}
