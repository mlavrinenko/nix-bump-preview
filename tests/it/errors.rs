use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use nix_bump_preview::cli::Cli;
use nix_bump_preview::cost;
use nix_bump_preview::delta;
use nix_bump_preview::eval;
use nix_bump_preview::nix::{Nix, NixCommand};
use temp_env::with_var;

use crate::common::{Harness, Response, assert_untouched, fixture, pin_mtime, temp_lock};

const SYSTEM: &str = "nixosConfigurations.example.config.system.build.toplevel";
const PACKAGE: &str = "packages.example";

fn assert_failure(error: &impl std::fmt::Display, expected: &[&str]) {
    let message = error.to_string();
    assert!(message.contains("exit status"), "missing status: {message}");
    for piece in expected {
        assert!(message.contains(piece), "missing {piece:?}: {message}");
    }
}

#[test]
fn a_failed_update_reports_argv_status_and_stderr() {
    let mut harness = Harness::new("errors-update", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.respond(1, &Response::failure("error: update exploded"));

    let error = harness.resolve().expect_err("a failed update must fail");

    assert_failure(
        &error,
        &["flake", "update", "--output-lock-file", "update exploded"],
    );
    assert!(!temp_lock(&harness.first_invocation()).exists());
}

#[test]
fn a_failed_eval_reports_argv_status_and_stderr() {
    let mut harness = Harness::new("errors-eval", "locks/before.lock");
    harness.cli.lock_file = Some(fixture("locks/after.lock"));
    harness.cli.installable = Vec::new();
    harness.cli.attr = vec![PACKAGE.to_owned()];
    harness.respond_with("eval", 1, &Response::failure("error: eval exploded"));
    let resolution = harness.resolve().expect("resolve should succeed");

    let error = eval::evaluate(
        &harness.nix(),
        &resolution,
        &harness.cli.installable,
        &harness.cli.attr,
    )
    .expect_err("a failed eval must fail");

    assert_failure(&error, &["eval", "--raw", "eval exploded"]);
}

#[test]
fn a_failed_dry_run_reports_argv_status_and_stderr() {
    let harness = Harness::new("errors-dry-run", "locks/before.lock");
    harness.respond_with("build", 1, &Response::failure("error: plan exploded"));
    let after = fixture("locks/after.lock");

    let error = cost::plan(&harness.nix(), harness.flake(), PACKAGE, &after)
        .expect_err("a failed build dry-run must fail");

    assert_failure(&error, &["build", "--dry-run", "plan exploded"]);
}

#[test]
fn a_failed_temp_removal_does_not_panic_and_leaves_the_path() {
    let mut harness = Harness::new("errors-failed-cleanup", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));

    let resolution = harness.resolve().expect("resolve should succeed");
    let temp = temp_lock(&harness.first_invocation());
    fs::remove_file(&temp).expect("the temp lock should be removable");
    fs::create_dir(&temp).expect("the temp path should become a directory");
    let marker = temp.join("marker");
    fs::write(&marker, "still here").expect("the marker should be writable");

    drop(resolution);

    assert!(temp.is_dir(), "a failed removal must leave the path alone");
    assert_eq!(
        fs::read_to_string(&marker).expect("the marker should stay readable"),
        "still here"
    );
    fs::remove_dir_all(&temp).expect("manual cleanup should succeed");
    assert!(!temp.exists());
}

#[test]
fn a_provided_lock_file_is_never_modified() {
    let mut harness = Harness::new("errors-lock-file", "locks/before.lock");
    let proposed = harness.dir.path().join("proposed.lock");
    fs::copy(fixture("locks/after.lock"), &proposed).expect("proposed lock should be copyable");
    let bytes = fs::read(&proposed).expect("proposed lock should be readable");
    let mtime = pin_mtime(&proposed);
    harness.cli.lock_file = Some(proposed.clone());

    let resolution = harness.resolve().expect("resolve should succeed");
    assert_eq!(resolution.after_lock(), proposed);
    assert!(harness.invocations().is_empty());
    drop(resolution);

    assert_untouched(&proposed, &bytes, mtime);
}

#[test]
fn an_explicit_missing_nix_bin_is_a_clear_error() {
    let cli = Cli::try_parse_from(["nix-bump-preview", "--nix-bin", "/missing"])
        .expect("flag should parse");

    let error = Nix::new(cli.nix_bin).expect_err("an explicit missing path must fail");

    let message = error.to_string();
    assert!(message.contains("nix executable not found"), "{message}");
    assert!(message.contains("/missing"), "{message}");
}

#[test]
fn a_bare_missing_nix_bin_from_the_environment_is_a_clear_error() {
    with_var("NIX_BIN", Some("nix-bump-preview-missing-nix"), || {
        let cli = Cli::try_parse_from(["nix-bump-preview"]).expect("the env should parse");
        assert_eq!(cli.nix_bin, PathBuf::from("nix-bump-preview-missing-nix"));
        let nix = Nix::new(cli.nix_bin).expect("a bare name stays a PATH lookup");

        let error = nix
            .run(&NixCommand::diff_closures(
                Path::new("/nix/store/before.drv"),
                Path::new("/nix/store/after.drv"),
            ))
            .expect_err("a missing binary must fail");

        let message = error.to_string();
        assert!(message.contains("failed to run"), "{message}");
        assert!(
            message.contains("nix-bump-preview-missing-nix"),
            "{message}"
        );
    });
}

#[test]
fn nix_bin_precedence_is_flag_then_environment_then_default() {
    with_var("NIX_BIN", None::<&str>, || {
        let cli = Cli::try_parse_from(["nix-bump-preview"]).expect("the default should parse");
        assert_eq!(cli.nix_bin, PathBuf::from("nix"));
    });
    with_var("NIX_BIN", Some("env-nix"), || {
        let from_env = Cli::try_parse_from(["nix-bump-preview"]).expect("the env should parse");
        assert_eq!(from_env.nix_bin, PathBuf::from("env-nix"));

        let from_flag = Cli::try_parse_from(["nix-bump-preview", "--nix-bin", "flag-nix"])
            .expect("the flag should parse");
        assert_eq!(from_flag.nix_bin, PathBuf::from("flag-nix"));
    });
}

fn assert_flag(args: &[String], flag: &str) {
    assert!(
        args.iter().any(|arg| arg == flag),
        "safe form is missing {flag}: {args:?}"
    );
}

fn assert_safe_form(args: &[String]) -> &'static str {
    for forbidden in ["switch", "boot", "test", "activate", "apply", "profile"] {
        assert!(
            !args.iter().any(|arg| arg == forbidden),
            "activation command in argv: {args:?}"
        );
    }
    match (
        args.first().map(String::as_str),
        args.get(1).map(String::as_str),
    ) {
        (Some("flake"), Some("update")) => {
            assert_flag(args, "--flake");
            assert_flag(args, "--output-lock-file");
            "flake update"
        }
        (Some("path-info"), _) => {
            assert_flag(args, "--derivation");
            assert_flag(args, "--no-write-lock-file");
            "path-info"
        }
        (Some("store"), Some("diff-closures")) => {
            assert_flag(args, "--derivation");
            "store diff-closures"
        }
        (Some("build"), _) => {
            assert_flag(args, "--dry-run");
            assert_flag(args, "--no-write-lock-file");
            "build"
        }
        (Some("eval"), _) => {
            assert_flag(args, "--raw");
            assert_flag(args, "--no-write-lock-file");
            "eval"
        }
        _ => panic!("undocumented nix invocation: {args:?}"),
    }
}

#[test]
fn every_stage_records_only_documented_safe_invocations() {
    let mut harness = Harness::new("errors-safety", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.cli.installable = vec![SYSTEM.to_owned()];
    harness.cli.attr = vec![PACKAGE.to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
    harness.respond_with("path-info", 1, &Response::stdout("/nix/store/before.drv\n"));
    harness.respond_with("path-info", 2, &Response::stdout("/nix/store/after.drv\n"));
    harness.respond_with("eval", 1, &Response::stdout("1.0\n"));
    harness.respond_with("eval", 2, &Response::stdout("2.0\n"));
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::stdout("pkg: 1.0 -> 2.0\n"),
    );
    harness.respond_with(
        "build",
        1,
        &Response::stderr("this derivation will be built:\n"),
    );

    let resolution = harness.resolve().expect("resolve should succeed");
    let evaluated = eval::evaluate(
        &harness.nix(),
        &resolution,
        &harness.cli.installable,
        &harness.cli.attr,
    )
    .expect("evaluation should succeed");
    let installable = evaluated
        .installables()
        .first()
        .expect("installable report should exist");
    delta::diff(
        &harness.nix(),
        Path::new(&installable.before.drv_path),
        Path::new(&installable.after.drv_path),
    )
    .expect("the delta should succeed");
    cost::plan(
        &harness.nix(),
        harness.flake(),
        SYSTEM,
        resolution.after_lock(),
    )
    .expect("the cost plan should succeed");

    let records = harness.invocations();
    let mut forms = BTreeSet::new();
    for args in &records {
        forms.insert(assert_safe_form(args));
    }

    assert_eq!(records.len(), 7, "every stage should have run");
    assert_eq!(
        forms,
        BTreeSet::from([
            "flake update",
            "path-info",
            "store diff-closures",
            "build",
            "eval",
        ])
    );
}
