use std::fs;
use std::path::{Path, PathBuf};

use nix_bump_preview::cost;
use nix_bump_preview::report::Cost;

use crate::common::{Harness, Response, canonical, fixture};

const INSTALLABLE: &str = "packages.example";

fn after_lock() -> PathBuf {
    fixture("locks/after.lock")
}

fn build_args(flake: &Path, installable: &str, after: &Path) -> Vec<String> {
    vec![
        "build".to_owned(),
        "--dry-run".to_owned(),
        "--no-write-lock-file".to_owned(),
        "--reference-lock-file".to_owned(),
        after.display().to_string(),
        format!("{}#{installable}", canonical(flake)),
    ]
}

fn plan(harness: &Harness, installable: &str, after: &Path) -> Cost {
    cost::plan(&harness.nix(), harness.flake(), installable, after)
        .expect("cost plan should succeed")
}

#[test]
fn runs_the_exact_dry_run_argv_and_parses_the_captured_output() {
    let harness = Harness::new("cost-captured", "locks/before.lock");
    let captured = fs::read_to_string(fixture("captured/dry-run.txt"))
        .expect("captured dry-run output should be readable");
    harness.respond_with("build", 1, &Response::stderr(&captured));
    let after = after_lock();

    let cost = plan(&harness, INSTALLABLE, &after);

    assert_eq!(cost.to_build, 1);
    assert_eq!(cost.to_fetch, 0);
    assert_eq!(cost.download_bytes, 0);
    assert_eq!(cost.unpacked_bytes, 0);
    assert_eq!(
        harness.invocations(),
        vec![build_args(harness.flake(), INSTALLABLE, &after)]
    );
}

#[test]
fn parses_plural_headers_from_stderr_and_ignores_stdout() {
    let harness = Harness::new("cost-plural", "locks/before.lock");
    harness.respond_with(
        "build",
        1,
        &Response::stdout("these 9 paths will be fetched (9 TiB download, 9 TiB unpacked):\n")
            .with_stderr(
                "\
these 721 derivations will be built:
  /nix/store/00000000000000000000000000000000-example.drv
these 671 paths will be fetched (3.0 GiB download, 8.4 GiB unpacked):
  /nix/store/00000000000000000000000000000000-example
",
            ),
    );

    let cost = plan(&harness, INSTALLABLE, &after_lock());

    assert_eq!(cost.to_build, 721);
    assert_eq!(cost.to_fetch, 671);
    assert_eq!(cost.download_bytes, 3_221_225_472);
    assert_eq!(cost.unpacked_bytes, 9_019_431_322);
}

#[test]
fn a_missing_header_is_zero() {
    let harness = Harness::new("cost-zero", "locks/before.lock");
    harness.respond_with("build", 1, &Response::stderr("warning: nothing to do\n"));

    let cost = plan(&harness, INSTALLABLE, &after_lock());

    assert_eq!(cost.to_build, 0);
    assert_eq!(cost.to_fetch, 0);
    assert_eq!(cost.download_bytes, 0);
    assert_eq!(cost.unpacked_bytes, 0);
}

#[test]
fn installables_are_not_merged() {
    let harness = Harness::new("cost-per-installable", "locks/before.lock");
    harness.respond_with(
        "build",
        1,
        &Response::stderr("these 2 derivations will be built:\n"),
    );
    harness.respond_with(
        "build",
        2,
        &Response::stderr("these 5 derivations will be built:\n"),
    );
    let after = after_lock();

    let first = plan(&harness, "packages.first", &after);
    let second = plan(&harness, "packages.second", &after);

    assert_eq!(first.to_build, 2);
    assert_eq!(second.to_build, 5);
    assert_eq!(
        harness.invocations(),
        vec![
            build_args(harness.flake(), "packages.first", &after),
            build_args(harness.flake(), "packages.second", &after),
        ]
    );
}

#[test]
fn a_malformed_header_is_a_clear_error() {
    let harness = Harness::new("cost-malformed", "locks/before.lock");
    harness.respond_with(
        "build",
        1,
        &Response::stderr("these lots derivations will be built:\n"),
    );

    let error = cost::plan(&harness.nix(), harness.flake(), INSTALLABLE, &after_lock())
        .expect_err("a malformed header must fail");
    let message = error.to_string();

    assert!(
        message.contains("malformed cost header"),
        "missing reason: {message}"
    );
    assert!(
        message.contains("these lots derivations"),
        "missing header: {message}"
    );
}

#[test]
fn a_failed_build_dry_run_reports_argv_status_and_stderr() {
    let harness = Harness::new("cost-failure", "locks/before.lock");
    harness.respond_with("build", 1, &Response::failure("error: plan exploded"));

    let error = cost::plan(&harness.nix(), harness.flake(), INSTALLABLE, &after_lock())
        .expect_err("a failed build dry-run must fail");
    let message = error.to_string();

    assert!(message.contains("build"), "missing argv: {message}");
    assert!(message.contains("--dry-run"), "missing argv: {message}");
    assert!(message.contains("exit status"), "missing status: {message}");
    assert!(
        message.contains("plan exploded"),
        "missing stderr: {message}"
    );
    assert_eq!(
        harness.invocations(),
        vec![build_args(harness.flake(), INSTALLABLE, &after_lock())]
    );
}

#[test]
fn every_recorded_invocation_is_dry_and_lock_free() {
    let harness = Harness::new("cost-safe", "locks/before.lock");
    harness.respond_with(
        "build",
        1,
        &Response::stderr("this derivation will be built:\n"),
    );

    plan(&harness, INSTALLABLE, &after_lock());

    for args in harness.invocations() {
        assert!(
            args.iter().any(|arg| arg == "--dry-run"),
            "unsafe argv without --dry-run: {args:?}"
        );
        assert!(
            args.iter().any(|arg| arg == "--no-write-lock-file"),
            "unsafe argv without --no-write-lock-file: {args:?}"
        );
    }
}
