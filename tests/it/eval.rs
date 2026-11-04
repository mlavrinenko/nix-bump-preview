use std::path::Path;

use nix_bump_preview::eval::{self, Evaluation};
use nix_bump_preview::lock::Resolution;

use crate::common::{Harness, Response, canonical, fixture, temp_lock};

const SYSTEM: &str = "nixosConfigurations.example.config.system.build.toplevel";
const SHELL: &str = "devShells.example";
const PACKAGE: &str = "packages.example";
const VERSION_ATTR: &str = "nixosConfigurations.example._module.args.pkgs.unstable.zed-editor";

fn harness_with_lock_file(label: &str) -> Harness {
    let mut harness = Harness::new(label, "locks/before.lock");
    harness.cli.lock_file = Some(fixture("locks/after.lock"));
    harness
}

fn evaluate(harness: &Harness, resolution: &Resolution) -> Evaluation {
    eval::evaluate(
        &harness.nix(),
        resolution,
        &harness.cli.installable,
        &harness.cli.attr,
    )
    .expect("evaluation should succeed")
}

fn nix_args(
    command: &str,
    flags: &[&str],
    flake: &Path,
    spec: &str,
    after: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![command.to_owned()];
    args.extend(flags.iter().map(|flag| (*flag).to_owned()));
    args.push("--no-write-lock-file".to_owned());
    if let Some(lock) = after {
        args.push("--reference-lock-file".to_owned());
        args.push(lock.display().to_string());
    }
    args.push(format!("{}#{spec}", canonical(flake)));
    args
}

fn path_info_args(flake: &Path, installable: &str, after: Option<&Path>) -> Vec<String> {
    nix_args("path-info", &["--derivation"], flake, installable, after)
}

fn eval_args(flake: &Path, attr: &str, after: Option<&Path>) -> Vec<String> {
    nix_args("eval", &["--raw"], flake, &format!("{attr}.version"), after)
}

#[test]
fn installables_use_the_exact_argv_for_both_locks_in_cli_order() {
    let mut harness = harness_with_lock_file("eval-installables");
    harness.cli.installable = vec![SHELL.to_owned(), SYSTEM.to_owned()];
    harness.respond_with(
        "path-info",
        1,
        &Response::stdout("/nix/store/before-shell.drv\n"),
    );
    harness.respond_with(
        "path-info",
        2,
        &Response::stdout("/nix/store/after-shell.drv\n"),
    );
    harness.respond_with(
        "path-info",
        3,
        &Response::stdout("/nix/store/before-system.drv\n"),
    );
    harness.respond_with(
        "path-info",
        4,
        &Response::stdout("/nix/store/after-system.drv\n"),
    );

    let resolution = harness.resolve().expect("resolve should succeed");
    let evaluated = evaluate(&harness, &resolution);
    let after = resolution.after_lock();

    assert_eq!(
        harness.invocations(),
        vec![
            path_info_args(harness.flake(), SHELL, None),
            path_info_args(harness.flake(), SHELL, Some(after)),
            path_info_args(harness.flake(), SYSTEM, None),
            path_info_args(harness.flake(), SYSTEM, Some(after)),
        ]
    );
    let reports = evaluated.installables();
    let attrs: Vec<&str> = reports.iter().map(|report| report.attr.as_str()).collect();
    assert_eq!(attrs, [SHELL, SYSTEM]);
    assert!(evaluated.attrs().is_empty());

    let shell = reports.first().expect("shell report should exist");
    assert_eq!(shell.before.drv_path, "/nix/store/before-shell.drv");
    assert_eq!(shell.after.drv_path, "/nix/store/after-shell.drv");
    assert!(shell.delta.is_none());
    assert!(shell.cost.is_none());

    let system = reports.get(1).expect("system report should exist");
    assert_eq!(system.before.drv_path, "/nix/store/before-system.drv");
    assert_eq!(system.after.drv_path, "/nix/store/after-system.drv");
}

#[test]
fn attrs_use_stdout_only_and_strip_only_trailing_newlines() {
    let mut harness = harness_with_lock_file("eval-attrs");
    harness.cli.attr = vec![PACKAGE.to_owned(), VERSION_ATTR.to_owned()];
    harness.respond_with(
        "path-info",
        1,
        &Response::stdout("/nix/store/before-system.drv\n"),
    );
    harness.respond_with(
        "path-info",
        2,
        &Response::stdout("/nix/store/after-system.drv\n"),
    );
    harness.respond_with(
        "eval",
        1,
        &Response::stdout("1.24.2\n").with_stderr("warning: diagnostics are not a version\n"),
    );
    harness.respond_with("eval", 2, &Response::stdout("1.25.2\n\n"));
    harness.respond_with("eval", 3, &Response::stdout("\n"));
    harness.respond_with("eval", 4, &Response::stdout("1.0\nstill here\n"));

    let resolution = harness.resolve().expect("resolve should succeed");
    let evaluated = evaluate(&harness, &resolution);
    let after = resolution.after_lock();

    assert_eq!(
        harness.invocations(),
        vec![
            path_info_args(
                harness.flake(),
                "nixosConfigurations.example.config.system.build.toplevel",
                None,
            ),
            path_info_args(
                harness.flake(),
                "nixosConfigurations.example.config.system.build.toplevel",
                Some(after),
            ),
            eval_args(harness.flake(), PACKAGE, None),
            eval_args(harness.flake(), PACKAGE, Some(after)),
            eval_args(harness.flake(), VERSION_ATTR, None),
            eval_args(harness.flake(), VERSION_ATTR, Some(after)),
        ]
    );
    let attrs = evaluated.attrs();
    let names: Vec<&str> = attrs.iter().map(|report| report.attr.as_str()).collect();
    assert_eq!(names, [PACKAGE, VERSION_ATTR]);

    let package = attrs.first().expect("package report should exist");
    assert_eq!(package.before, "1.24.2");
    assert_eq!(package.after, "1.25.2");

    let version = attrs.get(1).expect("version report should exist");
    assert_eq!(version.before, "");
    assert_eq!(version.after, "1.0\nstill here");
}

#[test]
fn installables_run_before_attrs_and_nothing_can_build_or_activate() {
    let mut harness = harness_with_lock_file("eval-safe");
    harness.cli.installable = vec![SHELL.to_owned(), SYSTEM.to_owned()];
    harness.cli.attr = vec![PACKAGE.to_owned()];
    for (index, path) in [
        (1, "/nix/store/before-shell.drv"),
        (2, "/nix/store/after-shell.drv"),
        (3, "/nix/store/before-system.drv"),
        (4, "/nix/store/after-system.drv"),
    ] {
        harness.respond_with("path-info", index, &Response::stdout(path));
    }
    harness.respond_with("eval", 1, &Response::stdout("1.0\n"));
    harness.respond_with("eval", 2, &Response::stdout("2.0\n"));

    let resolution = harness.resolve().expect("resolve should succeed");
    evaluate(&harness, &resolution);

    let records = harness.invocations();
    let commands: Vec<&str> = records
        .iter()
        .map(|args| {
            args.first()
                .map(String::as_str)
                .expect("recorded argv should not be empty")
        })
        .collect();
    assert_eq!(
        commands,
        [
            "path-info",
            "path-info",
            "path-info",
            "path-info",
            "eval",
            "eval"
        ]
    );

    for args in records {
        assert!(
            args.iter().any(|arg| arg == "--no-write-lock-file"),
            "unsafe argv without --no-write-lock-file: {args:?}"
        );
        assert!(
            !args.iter().any(|arg| arg == "build"),
            "unexpected build argv: {args:?}"
        );
        for forbidden in ["switch", "boot", "activate", "apply"] {
            assert!(
                !args.iter().any(|arg| arg == forbidden),
                "unexpected activation word in argv: {args:?}"
            );
        }
    }
}

#[test]
fn a_failed_path_info_aborts_names_the_argv_and_stderr_and_cleans_up() {
    let mut harness = Harness::new("eval-path-info-failure", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.cli.installable = vec![SHELL.to_owned(), SYSTEM.to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
    harness.respond_with(
        "path-info",
        1,
        &Response::failure("error: path-info exploded"),
    );

    let resolution = harness.resolve().expect("resolve should succeed");
    let temp = temp_lock(&harness.first_invocation());
    assert!(temp.exists());

    let error = eval::evaluate(
        &harness.nix(),
        &resolution,
        &harness.cli.installable,
        &harness.cli.attr,
    )
    .expect_err("a failing path-info must abort the run");
    let message = error.to_string();

    assert_eq!(harness.invocations().len(), 2);
    assert_eq!(
        harness.invocations().get(1),
        Some(&path_info_args(harness.flake(), SHELL, None))
    );
    assert!(message.contains("path-info"), "missing argv: {message}");
    assert!(message.contains("--derivation"), "missing argv: {message}");
    assert!(message.contains("exit status"), "missing status: {message}");
    assert!(
        message.contains("path-info exploded"),
        "missing stderr: {message}"
    );

    drop(resolution);
    assert!(!temp.exists(), "the temp lock is removed after the run");
}

#[test]
fn a_failed_eval_aborts_with_stderr_and_makes_no_later_calls() {
    let mut harness = harness_with_lock_file("eval-attr-failure");
    harness.cli.installable = vec![SHELL.to_owned()];
    harness.cli.attr = vec![PACKAGE.to_owned(), VERSION_ATTR.to_owned()];
    harness.respond_with(
        "path-info",
        1,
        &Response::stdout("/nix/store/before-shell.drv\n"),
    );
    harness.respond_with(
        "path-info",
        2,
        &Response::stdout("/nix/store/after-shell.drv\n"),
    );
    harness.respond_with(
        "eval",
        1,
        &Response::failure("error: attribute 'packages.example' missing"),
    );

    let resolution = harness.resolve().expect("resolve should succeed");
    let error = eval::evaluate(
        &harness.nix(),
        &resolution,
        &harness.cli.installable,
        &harness.cli.attr,
    )
    .expect_err("a failing eval must abort the run");
    let message = error.to_string();

    assert_eq!(harness.invocations().len(), 3);
    assert_eq!(
        harness.invocations().get(2),
        Some(&eval_args(harness.flake(), PACKAGE, None))
    );
    assert!(message.contains("eval"), "missing argv: {message}");
    assert!(message.contains("--raw"), "missing argv: {message}");
    assert!(
        message.contains("attribute 'packages.example' missing"),
        "missing stderr: {message}"
    );
}
