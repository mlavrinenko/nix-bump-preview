use std::fs;
use std::path::Path;

use nix_bump_preview::delta;
use nix_bump_preview::report::ChangeKind;

use crate::common::{Harness, Response, fixture};

const BEFORE: &str = "/nix/store/old.drv";
const AFTER: &str = "/nix/store/new.drv";

fn invocation() -> Vec<String> {
    ["store", "diff-closures", "--derivation", BEFORE, AFTER]
        .iter()
        .map(|arg| (*arg).to_owned())
        .collect()
}

fn diff(harness: &Harness) -> delta::DeltaOutcome {
    delta::diff(&harness.nix(), Path::new(BEFORE), Path::new(AFTER)).expect("delta should succeed")
}

#[test]
fn runs_the_exact_derivation_diff_and_parses_the_captured_output() {
    let harness = Harness::new("delta-captured", "locks/before.lock");
    let captured = fs::read_to_string(fixture("captured/diff-closures.txt"))
        .expect("captured output should be readable");
    harness.respond_with("store-diff-closures", 1, &Response::stdout(&captured));

    let outcome = diff(&harness);

    assert_eq!(outcome.delta.approximation, "build-graph");
    let changes: Vec<(&str, ChangeKind)> = outcome
        .delta
        .changes
        .iter()
        .map(|change| (change.name.as_str(), change.kind))
        .collect();
    assert_eq!(
        changes,
        [
            ("libfoo", ChangeKind::Changed),
            ("newpkg", ChangeKind::Added),
            ("oldpkg", ChangeKind::Removed),
        ]
    );
    assert!(outcome.warnings.is_empty());
    assert_eq!(harness.invocations(), vec![invocation()]);
}

#[test]
fn a_realised_after_output_still_appears_and_no_store_query_runs() {
    let harness = Harness::new("delta-realised", "locks/before.lock");
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::stdout("already-built: 1.0 -> 2.0, +12.3 MiB\n"),
    );

    let outcome = diff(&harness);

    assert_eq!(outcome.delta.changes.len(), 1);
    let change = outcome
        .delta
        .changes
        .first()
        .expect("the realised package should appear");
    assert_eq!(change.name, "already-built");
    assert_eq!(change.kind, ChangeKind::Changed);
    assert_eq!(change.before, ["1.0"]);
    assert_eq!(change.after, ["2.0"]);

    let invocations = harness.invocations();
    assert_eq!(invocations, vec![invocation()]);
    for args in invocations {
        assert_eq!(args.first().map(String::as_str), Some("store"));
        assert_eq!(args.get(1).map(String::as_str), Some("diff-closures"));
        assert!(
            args.iter().any(|arg| arg == "--derivation"),
            "unsafe argv without --derivation: {args:?}"
        );
        for forbidden in ["build", "path-info", "query", "ls", "cat", "realise"] {
            assert!(
                !args.iter().any(|arg| arg == forbidden),
                "unexpected store query in argv: {args:?}"
            );
        }
    }
}

#[test]
fn malformed_lines_are_returned_as_separate_warnings() {
    let harness = Harness::new("delta-warnings", "locks/before.lock");
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::stdout("pkg: 1.0 -> 1.1\nnot a line\n"),
    );

    let outcome = diff(&harness);

    assert_eq!(
        outcome.warnings,
        vec!["ignored malformed diff line: not a line".to_owned()]
    );
    assert_eq!(outcome.delta.changes.len(), 1);
    assert!(
        outcome
            .warnings
            .iter()
            .all(|warning| !warning.starts_with("warning: ")),
        "the parser must not add the text prefix: {:?}",
        outcome.warnings
    );
}

#[test]
fn a_failed_diff_reports_argv_and_stderr() {
    let harness = Harness::new("delta-failure", "locks/before.lock");
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::failure("error: diff exploded"),
    );

    let error = delta::diff(&harness.nix(), Path::new(BEFORE), Path::new(AFTER))
        .expect_err("a failed diff must be an error");
    let message = error.to_string();

    assert!(
        message.contains("store diff-closures"),
        "missing argv: {message}"
    );
    assert!(message.contains("--derivation"), "missing argv: {message}");
    assert!(
        message.contains("diff exploded"),
        "missing stderr: {message}"
    );
    assert_eq!(harness.invocations(), vec![invocation()]);
}
