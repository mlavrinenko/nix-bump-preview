//! Parse the package delta from the build-time derivation graph.

use std::collections::BTreeMap;
use std::path::Path;

use thiserror::Error;

use crate::nix::{Nix, NixCommand, NixError};
use crate::report::{Change, ChangeKind, Delta};

const APPROXIMATION: &str = "build-graph";
const NO_VERSION: [&str; 2] = ["∅", "ε"];
const SIZE_UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

/// The parsed build-graph delta and warnings about ignored lines.
#[derive(Debug)]
pub struct DeltaOutcome {
    /// Package changes, sorted by name.
    pub delta: Delta,
    /// Malformed input lines, in input order, without a `warning: ` prefix.
    pub warnings: Vec<String>,
}

/// Run `nix store diff-closures --derivation` and parse its stdout.
///
/// The command always carries `--derivation`, so it reads the derivation graph
/// without realizing the after output. An already-realised output therefore
/// still appears in the delta.
///
/// # Errors
///
/// Returns [`DeltaError::Nix`] with the failed argv, status, and stderr when the
/// Nix call fails.
pub fn diff(nix: &Nix, before_drv: &Path, after_drv: &Path) -> Result<DeltaOutcome, DeltaError> {
    let output = nix.run(&NixCommand::diff_closures(before_drv, after_drv))?;
    let (delta, warnings) = parse(&output.stdout);
    Ok(DeltaOutcome { delta, warnings })
}

fn parse(text: &str) -> (Delta, Vec<String>) {
    let mut packages: BTreeMap<String, Versions> = BTreeMap::new();
    let mut warnings = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(line) {
            ParsedLine::Change {
                name,
                before,
                after,
            } => {
                let entry = packages.entry(name).or_default();
                merge_versions(&mut entry.before, before);
                merge_versions(&mut entry.after, after);
            }
            ParsedLine::SizeOnly => {}
            ParsedLine::Malformed => {
                warnings.push(format!("ignored malformed diff line: {line}"));
            }
        }
    }
    let changes = packages
        .into_iter()
        .filter_map(|(name, versions)| build_change(name, versions))
        .collect();
    (
        Delta {
            approximation: APPROXIMATION.to_owned(),
            changes,
        },
        warnings,
    )
}

#[derive(Debug, Default)]
struct Versions {
    before: Vec<String>,
    after: Vec<String>,
}

enum ParsedLine {
    Change {
        name: String,
        before: Vec<String>,
        after: Vec<String>,
    },
    SizeOnly,
    Malformed,
}

fn parse_line(line: &str) -> ParsedLine {
    let Some((left, right)) = split_arrow(line) else {
        return if is_size_only(line) {
            ParsedLine::SizeOnly
        } else {
            ParsedLine::Malformed
        };
    };
    let Some((name, before)) = left.split_once(':') else {
        return ParsedLine::Malformed;
    };
    let name = name.trim();
    if name.is_empty() {
        return ParsedLine::Malformed;
    }
    let (Some(before), Some(after)) = (parse_versions(before), parse_versions(right)) else {
        return ParsedLine::Malformed;
    };
    ParsedLine::Change {
        name: name.to_owned(),
        before,
        after,
    }
}

fn split_arrow(line: &str) -> Option<(&str, &str)> {
    let index = line.find("->").into_iter().chain(line.find('→')).min()?;
    let (left, rest) = line.split_at(index);
    let right = rest
        .strip_prefix("->")
        .or_else(|| rest.strip_prefix('→'))
        .unwrap_or_default();
    Some((left, right))
}

fn parse_versions(side: &str) -> Option<Vec<String>> {
    let mut items: Vec<&str> = side.split(',').map(str::trim).collect();
    if items.last().is_some_and(|item| is_size_token(item)) {
        items.pop();
    }
    let mut versions = Vec::new();
    for item in items {
        if item.is_empty() || item.contains("->") || item.contains('→') {
            return None;
        }
        if NO_VERSION.contains(&item) {
            continue;
        }
        if is_size_token(item) {
            return None;
        }
        if !versions.iter().any(|version| version == item) {
            versions.push(item.to_owned());
        }
    }
    Some(versions)
}

fn is_size_only(line: &str) -> bool {
    let line = line.trim();
    if is_size_list(line) {
        return true;
    }
    let Some((name, sizes)) = line.split_once(':') else {
        return false;
    };
    !name.trim().is_empty() && is_size_list(sizes.trim())
}

fn is_size_list(text: &str) -> bool {
    text.split(',').all(is_size_token)
}

fn is_size_token(token: &str) -> bool {
    let token = token.trim();
    let rest = token
        .strip_prefix('+')
        .or_else(|| token.strip_prefix('-'))
        .unwrap_or(token);
    let Some((number, unit)) = rest.trim_start().split_once(char::is_whitespace) else {
        return false;
    };
    is_decimal(number) && SIZE_UNITS.contains(&unit.trim())
}

fn is_decimal(number: &str) -> bool {
    let mut parts = number.split('.');
    let Some(whole) = parts.next() else {
        return false;
    };
    let fraction = parts.next();
    if parts.next().is_some() || !is_digits(whole) {
        return false;
    }
    fraction.is_none_or(is_digits)
}

fn is_digits(part: &str) -> bool {
    !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit())
}

fn merge_versions(target: &mut Vec<String>, extra: Vec<String>) {
    for version in extra {
        if !target.contains(&version) {
            target.push(version);
        }
    }
}

fn same_versions(left: &[String], right: &[String]) -> bool {
    left.len() == right.len() && left.iter().all(|version| right.contains(version))
}

fn build_change(name: String, versions: Versions) -> Option<Change> {
    if same_versions(&versions.before, &versions.after) {
        return None;
    }
    let kind = if versions.before.is_empty() {
        ChangeKind::Added
    } else if versions.after.is_empty() {
        ChangeKind::Removed
    } else {
        ChangeKind::Changed
    };
    Some(Change {
        name,
        kind,
        before: versions.before,
        after: versions.after,
    })
}

/// A failure while reading the derivation-graph delta.
#[derive(Debug, Error)]
pub enum DeltaError {
    /// The `nix store diff-closures` call failed.
    #[error(transparent)]
    Nix(#[from] NixError),
}

#[cfg(test)]
#[path = "delta_tests.rs"]
mod tests;
