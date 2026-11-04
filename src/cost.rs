//! Parse the build and fetch plan from `nix build --dry-run`.

use std::path::Path;

use thiserror::Error;

use crate::nix::{Nix, NixCommand, NixError};
use crate::report::Cost;

/// Run `nix build --dry-run` for one installable and parse its plan.
///
/// The command always carries `--dry-run` and `--no-write-lock-file`, so it
/// reads the proposed lock without writing it and realizes no output.
///
/// # Errors
///
/// Returns [`CostError::Nix`] when Nix exits non-zero, and
/// [`CostError::Malformed`] or [`CostError::Overflow`] when a plan header
/// carries a bad value.
pub fn plan(
    nix: &Nix,
    flake: &Path,
    installable: &str,
    after_lock: &Path,
) -> Result<Cost, CostError> {
    let output = nix.run(&NixCommand::build_dry_run(flake, installable, after_lock))?;
    parse(&output.stderr)
}

fn parse(text: &str) -> Result<Cost, CostError> {
    let mut cost = Cost {
        to_build: 0,
        to_fetch: 0,
        download_bytes: 0,
        unpacked_bytes: 0,
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match parse_header(line)? {
            Some(Header::Build(count)) => {
                cost.to_build = sum_within_u64(cost.to_build, count, line)?;
            }
            Some(Header::Fetch {
                count,
                download,
                unpacked,
            }) => {
                cost.to_fetch = sum_within_u64(cost.to_fetch, count, line)?;
                cost.download_bytes = sum_within_u64(cost.download_bytes, download, line)?;
                cost.unpacked_bytes = sum_within_u64(cost.unpacked_bytes, unpacked, line)?;
            }
            None => {}
        }
    }
    Ok(cost)
}

enum Header {
    Build(u64),
    Fetch {
        count: u64,
        download: u64,
        unpacked: u64,
    },
}

fn parse_header(line: &str) -> Result<Option<Header>, CostError> {
    let Some(body) = plan_body(line) else {
        return Ok(None);
    };
    if let Some(build) = body.strip_suffix("will be built:") {
        let count = parse_plan_count(build, "derivation", "derivations", line)?;
        return Ok(Some(Header::Build(count)));
    }
    let Some((fetch, sizes)) = body.split_once("will be fetched") else {
        return Ok(None);
    };
    let count = parse_plan_count(fetch, "path", "paths", line)?;
    let (download, unpacked) = parse_sizes(sizes, line)?;
    Ok(Some(Header::Fetch {
        count,
        download,
        unpacked,
    }))
}

fn plan_body(line: &str) -> Option<&str> {
    line.strip_prefix("these ")
        .or_else(|| line.strip_prefix("this "))
}

fn parse_plan_count(
    body: &str,
    singular: &str,
    plural: &str,
    line: &str,
) -> Result<u64, CostError> {
    let body = body.trim();
    if body == singular {
        return Ok(1);
    }
    let Some((count, noun)) = body.rsplit_once(' ') else {
        return Err(malformed(line, "expected a count and a noun"));
    };
    if noun != singular && noun != plural {
        return Err(malformed(
            line,
            format!("expected `{singular}` or `{plural}`"),
        ));
    }
    count
        .parse::<u64>()
        .map_err(|_| malformed(line, format!("malformed count `{count}`")))
}

fn parse_sizes(tail: &str, line: &str) -> Result<(u64, u64), CostError> {
    let tail = tail.trim();
    let Some(tail) = tail
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix("):"))
    else {
        return Err(malformed(line, "expected `(<download>, <unpacked>):`"));
    };
    let Some((download, unpacked)) = tail.split_once(',') else {
        return Err(malformed(line, "expected download and unpacked sizes"));
    };
    let Some(download) = download.trim().strip_suffix(" download") else {
        return Err(malformed(line, "expected a download size"));
    };
    let Some(unpacked) = unpacked.trim().strip_suffix(" unpacked") else {
        return Err(malformed(line, "expected an unpacked size"));
    };
    Ok((parse_size(download, line)?, parse_size(unpacked, line)?))
}

fn parse_size(value: &str, line: &str) -> Result<u64, CostError> {
    let Some((number, unit)) = value.rsplit_once(' ') else {
        return Err(malformed(line, format!("malformed size `{value}`")));
    };
    let Some(multiplier) = unit_multiplier(unit.trim()) else {
        return Err(malformed(
            line,
            format!("unknown size unit `{}`", unit.trim()),
        ));
    };
    let Some((numerator, denominator)) = parse_decimal(number.trim()) else {
        return Err(malformed(line, format!("malformed size `{value}`")));
    };
    let scaled = numerator
        .checked_mul(multiplier)
        .ok_or_else(|| overflow(line))?;
    let doubled = scaled.checked_mul(2).ok_or_else(|| overflow(line))?;
    let divisor = denominator.checked_mul(2).ok_or_else(|| overflow(line))?;
    let rounded = doubled
        .checked_add(denominator)
        .and_then(|total| total.checked_div(divisor))
        .ok_or_else(|| overflow(line))?;
    u64::try_from(rounded).map_err(|_| overflow(line))
}

fn unit_multiplier(unit: &str) -> Option<u128> {
    match unit {
        "B" => Some(1),
        "KiB" => Some(1_024),
        "MiB" => Some(1_048_576),
        "GiB" => Some(1_073_741_824),
        "TiB" => Some(1_099_511_627_776),
        _ => None,
    }
}

fn parse_decimal(number: &str) -> Option<(u128, u128)> {
    let (whole, fraction) = match number.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (number, None),
    };
    if !is_digits(whole) {
        return None;
    }
    let mut numerator = parse_digits(whole)?;
    let mut denominator = 1_u128;
    if let Some(fraction) = fraction {
        if !is_digits(fraction) {
            return None;
        }
        for byte in fraction.bytes() {
            let digit = u128::from(byte - b'0');
            numerator = numerator.checked_mul(10)?.checked_add(digit)?;
            denominator = denominator.checked_mul(10)?;
        }
    }
    Some((numerator, denominator))
}

fn parse_digits(digits: &str) -> Option<u128> {
    let mut value = 0_u128;
    for byte in digits.bytes() {
        let digit = u128::from(byte - b'0');
        value = value.checked_mul(10)?.checked_add(digit)?;
    }
    Some(value)
}

fn is_digits(part: &str) -> bool {
    !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
}

fn sum_within_u64(current: u64, extra: u64, line: &str) -> Result<u64, CostError> {
    current.checked_add(extra).ok_or_else(|| overflow(line))
}

fn malformed(line: &str, reason: impl Into<String>) -> CostError {
    CostError::Malformed {
        line: line.to_owned(),
        reason: reason.into(),
    }
}

fn overflow(line: &str) -> CostError {
    CostError::Overflow {
        line: line.to_owned(),
    }
}

/// A failure while reading the build and fetch plan.
#[derive(Debug, Error)]
pub enum CostError {
    /// A plan header carries a value that cannot be parsed.
    #[error("malformed cost header `{line}`: {reason}")]
    Malformed {
        /// The header line.
        line: String,
        /// Why the value could not be parsed.
        reason: String,
    },

    /// A plan value does not fit in `u64`.
    #[error("cost value overflows u64 in `{line}`")]
    Overflow {
        /// The header line.
        line: String,
    },

    /// The `nix build --dry-run` call failed.
    #[error(transparent)]
    Nix(#[from] NixError),
}

#[cfg(test)]
#[path = "cost_tests.rs"]
mod tests;
