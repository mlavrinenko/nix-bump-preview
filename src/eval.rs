//! Evaluate installables and attrs against the before and proposed locks.

use std::path::Path;

use thiserror::Error;

use crate::lock::Resolution;
use crate::nix::{Nix, NixCommand, NixError};
use crate::report::{AttrReport, Derivation, InstallableReport};

/// Installable and attr results from one evaluation pass.
#[derive(Debug)]
pub struct Evaluation {
    installables: Vec<InstallableReport>,
    attrs: Vec<AttrReport>,
}

impl Evaluation {
    /// Installable results in CLI order.
    #[must_use]
    pub fn installables(&self) -> &[InstallableReport] {
        &self.installables
    }

    /// Attr results in CLI order.
    #[must_use]
    pub fn attrs(&self) -> &[AttrReport] {
        &self.attrs
    }
}

/// Evaluate `installables` and `attrs` against `resolution`.
///
/// Each installable becomes a `.drv` path read with the before lock and then
/// the proposed lock. Each attr becomes its `.version`, read the same way.
/// Installables run before attrs, and the first failure stops the run before
/// any later Nix call.
///
/// # Errors
///
/// Returns [`EvalError::Nix`] with the failed argv, status, and stderr when a
/// `nix path-info` or `nix eval` call fails.
pub fn evaluate(
    nix: &Nix,
    resolution: &Resolution,
    installables: &[String],
    attrs: &[String],
) -> Result<Evaluation, EvalError> {
    let flake = resolution.flake();
    let after_lock = resolution.after_lock();
    let installable_reports = installables
        .iter()
        .map(|installable| evaluate_installable(nix, flake, installable, after_lock))
        .collect::<Result<Vec<_>, _>>()?;
    let attr_reports = attrs
        .iter()
        .map(|attr| evaluate_attr(nix, flake, attr, after_lock))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Evaluation {
        installables: installable_reports,
        attrs: attr_reports,
    })
}

fn evaluate_installable(
    nix: &Nix,
    flake: &Path,
    installable: &str,
    after_lock: &Path,
) -> Result<InstallableReport, EvalError> {
    let before = nix_text(nix, &NixCommand::path_info(flake, installable, None))?;
    let after = nix_text(
        nix,
        &NixCommand::path_info(flake, installable, Some(after_lock)),
    )?;
    Ok(InstallableReport {
        attr: installable.to_owned(),
        before: Derivation { drv_path: before },
        after: Derivation { drv_path: after },
        delta: None,
        cost: None,
    })
}

fn evaluate_attr(
    nix: &Nix,
    flake: &Path,
    attr: &str,
    after_lock: &Path,
) -> Result<AttrReport, EvalError> {
    let before = nix_text(nix, &NixCommand::eval_version(flake, attr, None))?;
    let after = nix_text(
        nix,
        &NixCommand::eval_version(flake, attr, Some(after_lock)),
    )?;
    Ok(AttrReport {
        attr: attr.to_owned(),
        before,
        after,
    })
}

fn nix_text(nix: &Nix, command: &NixCommand) -> Result<String, EvalError> {
    let output = nix.run(command)?;
    Ok(strip_trailing_newlines(&output.stdout).to_owned())
}

fn strip_trailing_newlines(text: &str) -> &str {
    text.trim_end_matches('\n')
}

/// A failure while evaluating installables or attrs.
#[derive(Debug, Error)]
pub enum EvalError {
    /// A Nix call failed, with its argv, status, and stderr.
    #[error(transparent)]
    Nix(#[from] NixError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_trailing_newlines() {
        assert_eq!(strip_trailing_newlines("1.0\n\n"), "1.0");
        assert_eq!(
            strip_trailing_newlines("1.0\nstill here\n"),
            "1.0\nstill here"
        );
        assert_eq!(strip_trailing_newlines(""), "");
    }
}
