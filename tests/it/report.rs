use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

use crate::common::{Harness, Response, canonical, fixture};

const SYSTEM: &str = "nixosConfigurations.example.config.system.build.toplevel";
const PACKAGE: &str = "nixosConfigurations.example._module.args.pkgs.unstable.zed-editor";
const DELTA_WARNING: &str = "the delta is the build-time derivation graph, not the runtime closure";
const EXAMPLE_JSON: &str = include_str!("../fixtures/reports/example.json");

fn binary(harness: &Harness) -> Command {
    let mut command = Command::cargo_bin(env!("CARGO_PKG_NAME")).expect("binary should build");
    command
        .arg("--nix-bin")
        .arg(fixture("fake-nix"))
        .env("FAKE_NIX_SCENARIO", harness.scenario())
        .env("FAKE_NIX_LOG", harness.log_path());
    command
}

fn locked_command(harness: &Harness, lock: &Path) -> Command {
    let mut command = binary(harness);
    command
        .arg("--flake")
        .arg(harness.flake())
        .arg("--lock-file")
        .arg(lock)
        .arg("--installable")
        .arg(SYSTEM);
    command
}

fn respond_paths(harness: &Harness) {
    harness.respond_with("path-info", 1, &Response::stdout("/nix/store/before.drv\n"));
    harness.respond_with("path-info", 2, &Response::stdout("/nix/store/after.drv\n"));
}

fn respond_full(harness: &Harness) {
    respond_paths(harness);
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::stdout("libfoo: 1.0 -> 1.1\nnewpkg: ∅ -> 2.0\noldpkg: 1.0 -> ∅\nnot a line\n"),
    );
    harness.respond_with(
        "build",
        1,
        &Response::stderr(
            "these 721 derivations will be built:\n\
             these 671 paths will be fetched (3.0 GiB download, 8.4 GiB unpacked):\n",
        ),
    );
    harness.respond_with("eval", 1, &Response::stdout("1.24.2\n"));
    harness.respond_with("eval", 2, &Response::stdout("1.25.2\n"));
}

fn full_json(harness: &Harness) -> Vec<u8> {
    let assert = locked_command(harness, &fixture("locks/after.lock"))
        .arg("--attr")
        .arg(PACKAGE)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    assert.get_output().stdout.clone()
}

#[test]
fn text_report_lists_inputs_installables_changes_cost_attrs_and_warnings() {
    let mut harness = Harness::new("report-text", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned(), "unstable".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
    respond_full(&harness);

    let assert = binary(&harness)
        .arg("--flake")
        .arg(harness.flake())
        .arg("--input")
        .arg("nixpkgs")
        .arg("--input")
        .arg("unstable")
        .arg("--installable")
        .arg(SYSTEM)
        .arg("--attr")
        .arg(PACKAGE)
        .assert()
        .success();

    let expected = "\
input nixpkgs
  fe132422 (2026-10-15) -> 3ec63636 (2026-10-23)
input unstable: unchanged
installable nixosConfigurations.example.config.system.build.toplevel
  delta (build graph): 3 changes
    libfoo 1.0 -> 1.1
    newpkg ∅ -> 2.0
    oldpkg 1.0 -> ∅
  cost: 721 to build, 671 to fetch (3.0 GiB download, 8.4 GiB unpacked)
  attr nixosConfigurations.example._module.args.pkgs.unstable.zed-editor: 1.24.2 -> 1.25.2
warning: the delta is the build-time derivation graph, not the runtime closure
warning: ignored malformed diff line: not a line
";
    assert.stdout(expected);

    let invocations = harness.invocations();
    let commands: Vec<&str> = invocations
        .iter()
        .map(|args| args.first().map_or("", String::as_str))
        .collect();
    assert_eq!(
        commands,
        [
            "flake",
            "path-info",
            "path-info",
            "store",
            "build",
            "eval",
            "eval"
        ],
        "the run sequence must evaluate attrs after delta and cost"
    );
}

#[test]
fn json_report_matches_the_committed_fixture() {
    let harness = Harness::new("report-json", "locks/before.lock");
    respond_full(&harness);

    let stdout = full_json(&harness);
    let rendered = String::from_utf8(stdout).expect("report should be UTF-8");
    let normalized = rendered.replace(&canonical(harness.flake()), "/tmp/example");

    assert_eq!(normalized, EXAMPLE_JSON);
}

#[test]
fn json_report_validates_against_the_committed_schema() {
    let harness = Harness::new("report-schema", "locks/before.lock");
    respond_full(&harness);

    let stdout = full_json(&harness);
    let value: Value = serde_json::from_slice(&stdout).expect("report should be JSON");
    let schema: Value = serde_json::from_str(include_str!("../../schema/report.schema.json"))
        .expect("schema should be JSON");
    let validator = jsonschema::validator_for(&schema).expect("schema should compile");

    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "schema errors: {errors:?}");
}

#[test]
fn json_omits_uncomputed_and_empty_sections() {
    let harness = Harness::new("report-omitted", "locks/before.lock");
    respond_paths(&harness);

    let assert = locked_command(&harness, &fixture("locks/before.lock"))
        .arg("--no-delta")
        .arg("--no-cost")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let value: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("report should be JSON");

    assert!(value.get("attrs").is_none());
    assert!(value.get("warnings").is_none());
    let installable = value
        .get("installables")
        .and_then(Value::as_array)
        .and_then(|installables| installables.first())
        .expect("one installable should be reported");
    assert!(installable.get("delta").is_none());
    assert!(installable.get("cost").is_none());
    let local = value
        .get("inputs")
        .and_then(Value::as_array)
        .and_then(|inputs| inputs.first())
        .expect("local input should be reported");
    assert_eq!(local.get("name").and_then(Value::as_str), Some("local"));
    assert_eq!(
        local.get("before").and_then(|before| before.get("rev")),
        None
    );
}

#[test]
fn skip_flags_omit_their_calls_and_report_sections() {
    let harness = Harness::new("report-skips", "locks/before.lock");
    respond_paths(&harness);
    let assert = locked_command(&harness, &fixture("locks/before.lock"))
        .arg("--no-delta")
        .arg("--no-cost")
        .arg("--exit-code")
        .assert()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("text is UTF-8");
    assert!(!stdout.contains("delta"), "unexpected delta: {stdout}");
    assert!(!stdout.contains("cost:"), "unexpected cost: {stdout}");
    assert!(!stdout.contains("warning:"), "unexpected warning: {stdout}");

    let invocations = harness.invocations();
    assert_eq!(invocations.len(), 2, "only the two path-info calls run");
    for args in invocations {
        assert_eq!(args.first().map(String::as_str), Some("path-info"));
    }
}

#[test]
fn keep_lock_copies_the_generated_proposed_lock() {
    let mut harness = Harness::new("report-keep-lock", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
    respond_paths(&harness);
    let kept = harness.dir.path().join("kept/nested.lock");

    binary(&harness)
        .arg("--flake")
        .arg(harness.flake())
        .arg("--input")
        .arg("nixpkgs")
        .arg("--installable")
        .arg(SYSTEM)
        .arg("--keep-lock")
        .arg(&kept)
        .arg("--no-delta")
        .arg("--no-cost")
        .assert()
        .success();

    let expected = fs::read(fixture("locks/after.lock")).expect("fixture should be readable");
    assert_eq!(fs::read(&kept).expect("kept lock should exist"), expected);
    let first = harness.first_invocation();
    assert_eq!(first.first().map(String::as_str), Some("flake"));
    assert_eq!(first.get(1).map(String::as_str), Some("update"));
}

#[test]
fn a_failure_before_rendering_writes_nothing() {
    let harness = Harness::new("report-failure", "locks/before.lock");
    let missing = harness.dir.path().join("missing.lock");

    locked_command(&harness, &missing)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("cannot read"));
}

#[test]
fn exit_code_reports_a_changed_input() {
    let harness = Harness::new("exit-input", "locks/before.lock");
    respond_paths(&harness);

    locked_command(&harness, &fixture("locks/after.lock"))
        .arg("--no-delta")
        .arg("--no-cost")
        .arg("--exit-code")
        .assert()
        .code(1);
}

#[test]
fn exit_code_reports_a_delta_change() {
    let harness = Harness::new("exit-delta", "locks/before.lock");
    respond_paths(&harness);
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::stdout("pkg: 1.0 -> 2.0\n"),
    );
    harness.respond_with("build", 1, &Response::stderr(""));

    locked_command(&harness, &fixture("locks/before.lock"))
        .arg("--exit-code")
        .assert()
        .code(1);
}

#[test]
fn exit_code_reports_a_build_cost() {
    let harness = Harness::new("exit-cost", "locks/before.lock");
    respond_paths(&harness);
    harness.respond_with("store-diff-closures", 1, &Response::stdout(""));
    harness.respond_with(
        "build",
        1,
        &Response::stderr("this derivation will be built:\n"),
    );

    locked_command(&harness, &fixture("locks/before.lock"))
        .arg("--exit-code")
        .assert()
        .code(1);
}

#[test]
fn exit_code_is_zero_without_changes() {
    let harness = Harness::new("exit-clean", "locks/before.lock");
    respond_paths(&harness);
    harness.respond_with("store-diff-closures", 1, &Response::stdout(""));
    harness.respond_with("build", 1, &Response::stderr(""));

    locked_command(&harness, &fixture("locks/before.lock"))
        .arg("--exit-code")
        .assert()
        .code(0);
}

#[test]
fn no_delta_ignores_a_would_be_delta_change() {
    let harness = Harness::new("exit-no-delta", "locks/before.lock");
    respond_paths(&harness);
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::stdout("pkg: 1.0 -> 2.0\n"),
    );
    harness.respond_with("build", 1, &Response::stderr(""));

    locked_command(&harness, &fixture("locks/before.lock"))
        .arg("--no-delta")
        .arg("--exit-code")
        .assert()
        .code(0);

    for args in harness.invocations() {
        assert_ne!(args.first().map(String::as_str), Some("store"));
    }
}

#[test]
fn no_cost_ignores_a_would_be_cost() {
    let harness = Harness::new("exit-no-cost", "locks/before.lock");
    respond_paths(&harness);
    harness.respond_with("store-diff-closures", 1, &Response::stdout(""));
    harness.respond_with(
        "build",
        1,
        &Response::stderr("this derivation will be built:\n"),
    );

    locked_command(&harness, &fixture("locks/before.lock"))
        .arg("--no-cost")
        .arg("--exit-code")
        .assert()
        .code(0);

    for args in harness.invocations() {
        assert_ne!(args.first().map(String::as_str), Some("build"));
    }
}

#[test]
fn without_exit_code_success_is_zero() {
    let harness = Harness::new("exit-off", "locks/before.lock");
    respond_paths(&harness);

    locked_command(&harness, &fixture("locks/after.lock"))
        .arg("--no-delta")
        .arg("--no-cost")
        .assert()
        .code(0);
}

#[test]
fn the_delta_warning_is_exactly_the_documented_sentence() {
    let harness = Harness::new("report-warning", "locks/before.lock");
    respond_paths(&harness);
    harness.respond_with("store-diff-closures", 1, &Response::stdout(""));
    harness.respond_with("build", 1, &Response::stderr(""));

    let assert = locked_command(&harness, &fixture("locks/before.lock"))
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("text is UTF-8");

    assert!(stdout.contains(&format!("warning: {DELTA_WARNING}\n")));
}
