//! Preview a Nix flake input bump without building or switching.

pub mod cli;
pub mod cost;
pub mod delta;
pub mod eval;
pub mod lock;
pub mod nix;
pub mod report;

use std::io::{self, Write};
use std::path::Path;

use cli::{Cli, OutputFormat};
use lock::Resolution;
use nix::Nix;
use report::{InstallableReport, Report, SCHEMA_VERSION};

/// Run the preview for `cli` and return the process exit code.
///
/// # Errors
///
/// Returns an error when the arguments are invalid, when the schema cannot be
/// rendered, when a Nix call fails, or when the report cannot be written.
pub fn run(cli: &Cli) -> anyhow::Result<u8> {
    if cli.print_schema {
        let mut schema = report::schema_json()?;
        schema.push('\n');
        write_stdout(&schema)?;
        return Ok(0);
    }

    cli.validate()?;
    log::debug!(
        "preview requested for {} installable(s)",
        cli.installable.len()
    );
    let nix = Nix::new(&cli.nix_bin)?;
    let resolution = lock::resolve(cli, &nix)?;
    preview(cli, &nix, &resolution)
}

fn preview(cli: &Cli, nix: &Nix, resolution: &Resolution) -> anyhow::Result<u8> {
    let evaluated = eval::evaluate(nix, resolution, &cli.installable, &[])?;
    let mut installables = evaluated.installables().to_vec();
    let mut warnings = Vec::new();
    if !cli.no_delta {
        warnings.push(report::DELTA_WARNING.to_owned());
        run_deltas(nix, &mut installables, &mut warnings)?;
    }
    if !cli.no_cost {
        run_costs(nix, resolution, &mut installables)?;
    }
    let attr_eval = eval::evaluate(nix, resolution, &[], &cli.attr)?;
    let report = Report {
        schema_version: SCHEMA_VERSION,
        flake: resolution.flake().display().to_string(),
        inputs: resolution.inputs().to_vec(),
        installables,
        attrs: attr_eval.attrs().to_vec(),
        warnings,
    };

    let rendered = match cli.format {
        OutputFormat::Text => report::render_text(&report),
        OutputFormat::Json => report::render_json(&report)?,
    };
    write_stdout(&rendered)?;
    Ok(exit_code(cli, &report))
}

fn run_deltas(
    nix: &Nix,
    installables: &mut [InstallableReport],
    warnings: &mut Vec<String>,
) -> anyhow::Result<()> {
    for installable in installables {
        let outcome = delta::diff(
            nix,
            Path::new(&installable.before.drv_path),
            Path::new(&installable.after.drv_path),
        )?;
        warnings.extend(outcome.warnings);
        installable.delta = Some(outcome.delta);
    }
    Ok(())
}

fn run_costs(
    nix: &Nix,
    resolution: &Resolution,
    installables: &mut [InstallableReport],
) -> anyhow::Result<()> {
    for installable in installables {
        installable.cost = Some(cost::plan(
            nix,
            resolution.flake(),
            &installable.attr,
            resolution.after_lock(),
        )?);
    }
    Ok(())
}

fn exit_code(cli: &Cli, report: &Report) -> u8 {
    u8::from(cli.exit_code && has_changes(report))
}

fn has_changes(report: &Report) -> bool {
    report.inputs.iter().any(|input| input.changed)
        || report.installables.iter().any(installable_has_changes)
}

fn installable_has_changes(installable: &InstallableReport) -> bool {
    installable
        .delta
        .as_ref()
        .is_some_and(|delta| !delta.changes.is_empty())
        || installable
            .cost
            .as_ref()
            .is_some_and(|cost| cost.to_build > 0 || cost.to_fetch > 0)
}

fn write_stdout(text: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(text.as_bytes())?;
    handle.flush()
}
