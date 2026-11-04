use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::Serialize;

/// Version of the JSON report shape.
pub const SCHEMA_VERSION: u32 = 1;

/// The approximation warning text shared by both output formats.
pub const DELTA_WARNING: &str =
    "the delta is the build-time derivation graph, not the runtime closure";

/// A machine-readable preview report.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Version of the report shape; currently 1.
    pub schema_version: u32,
    /// Canonical path to the flake directory.
    pub flake: String,
    /// Selected inputs, sorted by name.
    pub inputs: Vec<InputReport>,
    /// Installables in CLI order.
    pub installables: Vec<InstallableReport>,
    /// Attrs in CLI order; omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<AttrReport>,
    /// Non-fatal problems; omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// One selected input and its resolved lock entries.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputReport {
    /// Input name.
    pub name: String,
    /// Whether any resolved field differs between the two locks.
    pub changed: bool,
    /// Entry in the before lock.
    pub before: Locked,
    /// Entry in the after lock.
    pub after: Locked,
}

/// Resolved fields of a lock entry; absent fields are omitted.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Locked {
    /// Resolved revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rev: Option<String>,
    /// Resolution timestamp as a Unix timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub last_modified: Option<u64>,
    /// NAR hash of the resolved source.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nar_hash: Option<String>,
}

/// One installable and its derivation preview.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstallableReport {
    /// Installable attr.
    pub attr: String,
    /// Derivation evaluated against the before lock.
    pub before: Derivation,
    /// Derivation evaluated against the after lock.
    pub after: Derivation,
    /// Package delta; omitted when not computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Delta")]
    pub delta: Option<Delta>,
    /// Build and fetch cost; omitted when not computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Cost")]
    pub cost: Option<Cost>,
}

/// A derivation path evaluated against one lock.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Derivation {
    /// Store path of the `.drv` file.
    pub drv_path: String,
}

/// Package version changes from the build-time derivation graph.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Delta {
    /// Name of the approximation used for the delta.
    pub approximation: String,
    /// Changed packages, sorted by name.
    pub changes: Vec<Change>,
}

/// One package whose version set changed.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// Package name.
    pub name: String,
    /// What happened to the package.
    pub kind: ChangeKind,
    /// Versions in the before lock.
    pub before: Vec<String>,
    /// Versions in the after lock.
    pub after: Vec<String>,
}

/// Classification of a package change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    /// The package appeared in the after graph.
    Added,
    /// The package disappeared from the after graph.
    Removed,
    /// The package stayed but its versions differ.
    Changed,
}

/// What the proposed lock would build and fetch.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    /// Derivations to build.
    pub to_build: u64,
    /// Paths to fetch.
    pub to_fetch: u64,
    /// Download size in bytes.
    pub download_bytes: u64,
    /// Unpacked size in bytes.
    pub unpacked_bytes: u64,
}

/// Direct version report for one attr.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttrReport {
    /// Attr whose `.version` was evaluated.
    pub attr: String,
    /// Version under the before lock.
    pub before: String,
    /// Version under the after lock.
    pub after: String,
}

/// Render the report JSON schema as pretty-printed JSON.
///
/// # Errors
///
/// Returns an error if the generated schema cannot be serialized.
pub fn schema_json() -> serde_json::Result<String> {
    let schema = schemars::generate::SchemaSettings::default()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<Report>();
    serde_json::to_string_pretty(&schema)
}

/// Render a report as one pretty-printed JSON object with a trailing newline.
///
/// # Errors
///
/// Returns an error if the report cannot be serialized.
pub fn render_json(report: &Report) -> serde_json::Result<String> {
    let mut json = serde_json::to_string_pretty(report)?;
    json.push('\n');
    Ok(json)
}

/// Render a report as the human-readable text contract in `docs/report.md`.
#[must_use]
pub fn render_text(report: &Report) -> String {
    let mut out = String::new();
    render_inputs(&mut out, &report.inputs);
    render_installables(&mut out, &report.installables);
    for attr in &report.attrs {
        let _ = writeln!(
            out,
            "  attr {}: {} -> {}",
            attr.attr, attr.before, attr.after
        );
    }
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    out
}

fn render_inputs(out: &mut String, inputs: &[InputReport]) {
    for input in inputs {
        if input.changed {
            let _ = writeln!(out, "input {}", input.name);
            let _ = writeln!(
                out,
                "  {} -> {}",
                render_locked(&input.before),
                render_locked(&input.after)
            );
        } else {
            let _ = writeln!(out, "input {}: unchanged", input.name);
        }
    }
}

fn render_installables(out: &mut String, installables: &[InstallableReport]) {
    for installable in installables {
        let _ = writeln!(out, "installable {}", installable.attr);
        render_delta(out, installable.delta.as_ref());
        render_cost(out, installable.cost.as_ref());
    }
}

fn render_delta(out: &mut String, delta: Option<&Delta>) {
    let Some(delta) = delta else {
        return;
    };
    let _ = writeln!(
        out,
        "  delta (build graph): {} changes",
        delta.changes.len()
    );
    for change in &delta.changes {
        let _ = writeln!(
            out,
            "    {} {} -> {}",
            change.name,
            render_versions(&change.before),
            render_versions(&change.after)
        );
    }
}

fn render_versions(versions: &[String]) -> String {
    if versions.is_empty() {
        "∅".to_owned()
    } else {
        versions.join(", ")
    }
}

fn render_cost(out: &mut String, cost: Option<&Cost>) {
    let Some(cost) = cost else {
        return;
    };
    let _ = writeln!(
        out,
        "  cost: {} to build, {} to fetch ({} download, {} unpacked)",
        cost.to_build,
        cost.to_fetch,
        format_size(cost.download_bytes),
        format_size(cost.unpacked_bytes)
    );
}

fn render_locked(locked: &Locked) -> String {
    let revision = locked
        .rev
        .as_deref()
        .or(locked.nar_hash.as_deref())
        .map_or_else(|| "?".to_owned(), first_eight);
    match locked.last_modified {
        Some(timestamp) => format!("{revision} ({})", format_date(timestamp)),
        None => revision,
    }
}

fn first_eight(text: &str) -> String {
    text.chars().take(8).collect()
}

fn format_date(timestamp: u64) -> String {
    let seconds = i64::try_from(timestamp).unwrap_or(i64::MAX);
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_position + 2) / 5 + 1;
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    };
    let leap_adjust = i64::from(month <= 2);
    (
        year + leap_adjust,
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

fn format_size(bytes: u64) -> String {
    const UNITS: [(&str, u64); 5] = [
        ("TiB", 1_u64 << 40),
        ("GiB", 1_u64 << 30),
        ("MiB", 1_u64 << 20),
        ("KiB", 1_u64 << 10),
        ("B", 1),
    ];
    let (unit, factor) = UNITS
        .iter()
        .find(|(_, factor)| bytes >= *factor)
        .copied()
        .unwrap_or(("B", 1));
    let tenths = (u128::from(bytes) * 10 + u128::from(factor) / 2) / u128::from(factor);
    format!("{}.{} {unit}", tenths / 10, tenths % 10)
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
