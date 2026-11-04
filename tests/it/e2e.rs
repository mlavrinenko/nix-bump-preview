use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

use crate::common::e2e::{
    ALPHA, BETA, FlakeSpec, drv, harness_for, locked_command, pin_before_lock, respond_full,
    run_locked,
};
use crate::common::{
    Response, assert_safe_form, assert_untouched, canonical, pin_mtime, temp_lock,
};

const SPECS: [&FlakeSpec; 2] = [&ALPHA, &BETA];
const DELTA_WARNING: &str = "the delta is the build-time derivation graph, not the runtime closure";

#[test]
fn text_reports_both_flakes_with_a_proposed_lock() {
    for spec in SPECS {
        let (_harness, stdout) = run_locked(spec, "text");
        let changed = format!("input {}\n", spec.changed);
        let unchanged = format!("input {}: unchanged", spec.unchanged);
        for piece in [
            changed.as_str(),
            unchanged.as_str(),
            spec.installable,
            "  delta (build graph): 1 changes",
            "  cost: 2 to build, 1 to fetch (1.5 KiB download, 2.0 KiB unpacked)",
            DELTA_WARNING,
        ] {
            assert!(stdout.contains(piece), "missing {piece:?} in:\n{stdout}");
        }
        assert!(stdout.contains(&format!("    {}-pkg 1.0 -> 2.0", spec.name)));
    }
}

#[test]
fn json_reports_both_flakes_with_a_proposed_lock() {
    for spec in SPECS {
        let (harness, stdout) = run_locked(spec, "json");
        let value: Value = serde_json::from_str(&stdout).expect("report should be JSON");
        let flake = canonical(harness.flake());
        assert_eq!(
            value.get("flake").and_then(Value::as_str),
            Some(flake.as_str())
        );
        let inputs = value
            .get("inputs")
            .and_then(Value::as_array)
            .expect("inputs should be an array");
        let names: Vec<&str> = inputs
            .iter()
            .filter_map(|input| input.get("name").and_then(Value::as_str))
            .collect();
        assert_eq!(names, [spec.changed, spec.unchanged]);
        let changed = inputs
            .iter()
            .find(|input| input.get("name").and_then(Value::as_str) == Some(spec.changed))
            .expect("the changed input should be reported");
        assert_eq!(changed.get("changed").and_then(Value::as_bool), Some(true));
        let unchanged = inputs
            .iter()
            .find(|input| input.get("name").and_then(Value::as_str) == Some(spec.unchanged))
            .expect("the unchanged input should be reported");
        assert_eq!(
            unchanged.get("changed").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            value
                .pointer("/installables/0/attr")
                .and_then(Value::as_str),
            Some(spec.installable)
        );
        let package = format!("{}-pkg", spec.name);
        assert_eq!(
            value
                .pointer("/installables/0/delta/changes/0/name")
                .and_then(Value::as_str),
            Some(package.as_str())
        );
        assert_eq!(
            value
                .pointer("/installables/0/cost/toBuild")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            value
                .pointer("/installables/0/cost/toFetch")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            value
                .pointer("/installables/0/cost/downloadBytes")
                .and_then(Value::as_u64),
            Some(1536)
        );
        assert_eq!(
            value.pointer("/warnings/0").and_then(Value::as_str),
            Some(DELTA_WARNING)
        );
        assert!(value.get("attrs").is_none());
    }
}

#[test]
fn all_updates_every_input_and_cleans_up_the_generated_lock() {
    let harness = harness_for(&ALPHA);
    let (lock, bytes, mtime) = pin_before_lock(&harness);
    respond_full(&harness, &ALPHA);
    let assert = harness
        .command()
        .arg("--flake")
        .arg(harness.flake())
        .arg("--all")
        .arg("--installable")
        .arg(ALPHA.installable)
        .arg("--no-delta")
        .arg("--no-cost")
        .assert()
        .success();

    let update = harness.first_invocation();
    assert_eq!(update.first().map(String::as_str), Some("flake"));
    assert_eq!(update.get(1).map(String::as_str), Some("update"));
    assert!(!update.iter().any(|arg| arg == ALPHA.changed));
    assert!(!update.iter().any(|arg| arg == ALPHA.unchanged));
    let temp = temp_lock(&update);
    assert!(!temp.exists(), "the generated lock should be removed");
    assert_untouched(&lock, &bytes, mtime);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is UTF-8");
    assert!(
        stdout.contains(&format!("input {}\n", ALPHA.changed)),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("input {}: unchanged", ALPHA.unchanged)),
        "{stdout}"
    );
}

#[test]
fn keep_lock_copies_the_generated_lock_and_removes_the_temp() {
    let harness = harness_for(&BETA);
    let (lock, bytes, mtime) = pin_before_lock(&harness);
    respond_full(&harness, &BETA);
    let kept = harness.dir.path().join("nested/proposed.lock");
    harness
        .command()
        .arg("--flake")
        .arg(harness.flake())
        .arg("--input")
        .arg(BETA.changed)
        .arg("--installable")
        .arg(BETA.installable)
        .arg("--keep-lock")
        .arg(&kept)
        .arg("--no-delta")
        .arg("--no-cost")
        .assert()
        .success();

    let expected = fs::read(harness.flake().join("proposed.lock")).expect("proposed lock");
    assert_eq!(fs::read(&kept).expect("kept lock should exist"), expected);
    let temp = temp_lock(&harness.first_invocation());
    assert!(!temp.exists(), "the generated lock should still be removed");
    assert_untouched(&lock, &bytes, mtime);
}

#[test]
fn no_delta_omits_the_diff_closures_call() {
    let harness = harness_for(&ALPHA);
    respond_full(&harness, &ALPHA);
    let proposed = harness.flake().join("proposed.lock");
    let assert = locked_command(&harness, &ALPHA, &proposed)
        .arg("--no-delta")
        .assert()
        .success();

    assert!(
        harness
            .invocations()
            .iter()
            .all(|args| args.first().map(String::as_str) != Some("store"))
    );
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is UTF-8");
    assert!(!stdout.contains("delta"), "{stdout}");
    assert!(stdout.contains("cost:"), "{stdout}");
}

#[test]
fn no_cost_omits_the_build_dry_run_call() {
    let harness = harness_for(&BETA);
    respond_full(&harness, &BETA);
    let proposed = harness.flake().join("proposed.lock");
    let assert = locked_command(&harness, &BETA, &proposed)
        .arg("--no-cost")
        .assert()
        .success();

    assert!(
        harness
            .invocations()
            .iter()
            .all(|args| args.first().map(String::as_str) != Some("build"))
    );
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is UTF-8");
    assert!(stdout.contains("delta (build graph):"), "{stdout}");
    assert!(!stdout.contains("cost:"), "{stdout}");
}

#[test]
fn exit_code_is_one_for_a_change_and_zero_without() {
    let changed = harness_for(&ALPHA);
    respond_full(&changed, &ALPHA);
    let proposed = changed.flake().join("proposed.lock");
    locked_command(&changed, &ALPHA, &proposed)
        .arg("--no-delta")
        .arg("--no-cost")
        .arg("--exit-code")
        .assert()
        .code(1);

    let unchanged = harness_for(&ALPHA);
    respond_full(&unchanged, &ALPHA);
    let before = unchanged.flake().join("flake.lock");
    locked_command(&unchanged, &ALPHA, &before)
        .arg("--no-delta")
        .arg("--no-cost")
        .arg("--exit-code")
        .assert()
        .code(0);
}

#[test]
fn a_failed_update_exits_2_keeps_the_lock_and_writes_no_report() {
    let harness = harness_for(&ALPHA);
    let (lock, bytes, mtime) = pin_before_lock(&harness);
    harness.respond(1, &Response::failure("error: update exploded"));

    harness
        .command()
        .arg("--flake")
        .arg(harness.flake())
        .arg("--input")
        .arg(ALPHA.changed)
        .arg("--installable")
        .arg(ALPHA.installable)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("update exploded"));

    assert_untouched(&lock, &bytes, mtime);
    let temp = temp_lock(&harness.first_invocation());
    assert!(!temp.exists(), "the generated lock should be removed");
}

#[test]
fn a_failed_path_info_exits_2_keeps_the_lock_and_writes_no_report() {
    let harness = harness_for(&BETA);
    let (lock, bytes, mtime) = pin_before_lock(&harness);
    let proposed = harness.flake().join("proposed.lock");
    harness.respond(1, &Response::success(&proposed));
    harness.respond_with(
        "path-info",
        1,
        &Response::failure("error: path-info exploded"),
    );

    harness
        .command()
        .arg("--flake")
        .arg(harness.flake())
        .arg("--input")
        .arg(BETA.changed)
        .arg("--installable")
        .arg(BETA.installable)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("path-info exploded"));

    assert_untouched(&lock, &bytes, mtime);
    let temp = temp_lock(&harness.first_invocation());
    assert!(!temp.exists(), "the generated lock should be removed");
}

#[test]
fn a_failed_diff_closures_exits_2_with_empty_stdout() {
    let harness = harness_for(&ALPHA);
    harness.respond_with("path-info", 1, &Response::stdout(&drv(&ALPHA, "before")));
    harness.respond_with("path-info", 2, &Response::stdout(&drv(&ALPHA, "after")));
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::failure("error: diff exploded"),
    );
    let proposed = harness.flake().join("proposed.lock");

    locked_command(&harness, &ALPHA, &proposed)
        .arg("--no-cost")
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("diff exploded"));
}

#[test]
fn a_failed_build_dry_run_exits_2_with_empty_stdout() {
    let harness = harness_for(&BETA);
    harness.respond_with("path-info", 1, &Response::stdout(&drv(&BETA, "before")));
    harness.respond_with("path-info", 2, &Response::stdout(&drv(&BETA, "after")));
    harness.respond_with("build", 1, &Response::failure("error: plan exploded"));
    let proposed = harness.flake().join("proposed.lock");

    locked_command(&harness, &BETA, &proposed)
        .arg("--no-delta")
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("plan exploded"));
}

#[test]
fn a_missing_flake_or_lock_exits_2_with_empty_stdout() {
    let harness = harness_for(&ALPHA);
    let proposed = harness.flake().join("proposed.lock");
    let absent = harness.dir.path().join("absent");
    harness
        .command()
        .arg("--flake")
        .arg(&absent)
        .arg("--lock-file")
        .arg(&proposed)
        .arg("--installable")
        .arg(ALPHA.installable)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("does not exist"));

    let bare = harness.dir.path().join("bare");
    fs::create_dir_all(&bare).expect("bare directory should be creatable");
    harness
        .command()
        .arg("--flake")
        .arg(&bare)
        .arg("--lock-file")
        .arg(&proposed)
        .arg("--installable")
        .arg(ALPHA.installable)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("no flake.lock"));
}

#[test]
fn a_missing_nix_binary_exits_2_with_empty_stdout() {
    let harness = harness_for(&ALPHA);
    let proposed = harness.flake().join("proposed.lock");
    let missing = harness.dir.path().join("missing-nix");

    harness
        .command_with_nix(&missing)
        .arg("--flake")
        .arg(harness.flake())
        .arg("--lock-file")
        .arg(&proposed)
        .arg("--installable")
        .arg(ALPHA.installable)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("nix executable not found"));
}

#[test]
fn every_argv_from_a_full_binary_run_is_safe() {
    let harness = harness_for(&ALPHA);
    respond_full(&harness, &ALPHA);

    harness
        .command()
        .arg("--flake")
        .arg(harness.flake())
        .arg("--input")
        .arg(ALPHA.changed)
        .arg("--installable")
        .arg(ALPHA.installable)
        .arg("--attr")
        .arg(ALPHA.attr)
        .assert()
        .success();

    let records = harness.invocations();
    assert_eq!(records.len(), 7, "every stage should have run");
    let forms: BTreeSet<&str> = records.iter().map(|args| assert_safe_form(args)).collect();
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

#[test]
#[ignore = "needs real Nix and a local flake; set NIX_BMP_REAL_FLAKE to run"]
fn real_nix_previews_a_local_flake_after_the_gate() {
    let Ok(flake) = std::env::var("NIX_BMP_REAL_FLAKE") else {
        eprintln!("skipping real-Nix test: NIX_BMP_REAL_FLAKE is not set");
        return;
    };
    let installable =
        std::env::var("NIX_BMP_REAL_INSTALLABLE").unwrap_or_else(|_| "packages.example".to_owned());
    let flake = PathBuf::from(flake);
    let lock = flake.join("flake.lock");
    let bytes = fs::read(&lock).expect("real flake.lock should be readable");
    let mtime = pin_mtime(&lock);

    let assert = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("binary should build")
        .arg("--flake")
        .arg(&flake)
        .arg("--lock-file")
        .arg(&lock)
        .arg("--installable")
        .arg(&installable)
        .arg("--no-delta")
        .arg("--no-cost")
        .assert()
        .success();

    assert_untouched(&lock, &bytes, mtime);
    assert!(
        !assert.get_output().stdout.is_empty(),
        "the preview should print a report"
    );
}
