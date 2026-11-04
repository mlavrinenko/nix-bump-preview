use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use thiserror::Error;

/// Preview a Nix flake input bump: package version deltas and build cost, without building or switching
#[derive(Debug, Parser)]
#[command(version, about)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each boolean is an independent flag in the documented CLI contract"
)]
pub struct Cli {
    /// Local flake directory containing flake.lock
    #[arg(short = 'f', long, default_value = ".")]
    pub flake: PathBuf,

    /// Root input to update; repeatable
    #[arg(short = 'i', long)]
    pub input: Vec<String>,

    /// Update every root input; exclusive with --input
    #[arg(long)]
    pub all: bool,

    /// Use an existing proposed lock; skip `nix flake update`
    #[arg(short = 'l', long)]
    pub lock_file: Option<PathBuf>,

    /// Attr under the flake to diff; repeatable, required
    #[arg(long)]
    pub installable: Vec<String>,

    /// Attr with a `.version` for a direct version delta; repeatable
    #[arg(long)]
    pub attr: Vec<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Copy the generated proposed lock to PATH
    #[arg(long)]
    pub keep_lock: Option<PathBuf>,

    /// Skip the package delta and its Nix call
    #[arg(long)]
    pub no_delta: bool,

    /// Skip the build and fetch cost and its Nix call
    #[arg(long)]
    pub no_cost: bool,

    /// Exit 1 when a change or build cost exists, else 0
    #[arg(long)]
    pub exit_code: bool,

    /// Nix executable, default `nix`
    #[arg(long, hide = true, env = "NIX_BIN", default_value = "nix")]
    pub nix_bin: PathBuf,

    /// Print the JSON schema and exit 0
    #[arg(long, hide = true)]
    pub print_schema: bool,
}

/// Output format for the report.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text.
    #[default]
    Text,
    /// Machine-readable JSON.
    Json,
}

/// Invalid command-line combinations.
#[derive(Debug, Error)]
pub enum CliError {
    /// No installable attr was given.
    #[error("at least one --installable is required")]
    MissingInstallable,

    /// No root input was selected and no proposed lock was given.
    #[error("either --input or --all is required unless --lock-file is given")]
    MissingInput,

    /// Both `--input` and `--all` were given.
    #[error("--input and --all cannot be used together")]
    InputAndAll,

    /// A proposed lock was given together with a root input selection.
    #[error("--lock-file cannot be used with --input or --all")]
    LockFileWithInput,

    /// A keep-lock destination was given together with a proposed lock.
    #[error("--keep-lock cannot be used with --lock-file")]
    KeepLockWithLockFile,
}

impl Cli {
    /// Check the business rules that clap cannot express without blocking
    /// `--print-schema`.
    ///
    /// # Errors
    ///
    /// Returns a [`CliError`] naming the first invalid combination found.
    pub fn validate(&self) -> Result<(), CliError> {
        if self.installable.is_empty() {
            return Err(CliError::MissingInstallable);
        }
        if self.all && !self.input.is_empty() {
            return Err(CliError::InputAndAll);
        }
        if self.lock_file.is_some() && (self.all || !self.input.is_empty()) {
            return Err(CliError::LockFileWithInput);
        }
        if self.lock_file.is_some() && self.keep_lock.is_some() {
            return Err(CliError::KeepLockWithLockFile);
        }
        if self.lock_file.is_none() && self.input.is_empty() && !self.all {
            return Err(CliError::MissingInput);
        }
        Ok(())
    }
}
