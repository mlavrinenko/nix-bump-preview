use std::fs;
use std::path::{Path, PathBuf};

use nix_bump_preview::lock;
use nix_bump_preview::nix::Nix;

use crate::common::{
    Harness, Response, assert_untouched, canonical, cli_for, diff_command, fixture, input_names,
    pin_mtime, temp_lock,
};

fn update_args(inputs: &[&str], flake: &Path, temp: &Path) -> Vec<String> {
    let mut args = vec!["flake".to_owned(), "update".to_owned()];
    args.extend(inputs.iter().map(|input| (*input).to_owned()));
    args.push("--flake".to_owned());
    args.push(canonical(flake));
    args.push("--output-lock-file".to_owned());
    args.push(temp.display().to_string());
    args
}

fn prepare_keep_lock(harness: &mut Harness, kept: &Path) {
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.cli.keep_lock = Some(kept.to_path_buf());
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
}

fn assert_kept_lock(kept: &Path) {
    let expected = fs::read(fixture("locks/after.lock")).expect("fixture should be readable");
    assert_eq!(
        fs::read(kept).expect("kept lock should be readable"),
        expected
    );
}

#[test]
fn input_selection_uses_the_exact_flake_update_argv() {
    let mut harness = Harness::new("argv-input", "locks/before.lock");
    harness.cli.input = vec!["unstable".to_owned(), "nixpkgs".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));

    let resolution = harness.resolve().expect("resolve should succeed");
    let args = harness.first_invocation();
    let temp = temp_lock(&args);

    assert_eq!(
        args,
        update_args(&["nixpkgs", "unstable"], harness.flake(), &temp)
    );
    assert_eq!(
        resolution.flake(),
        fs::canonicalize(harness.flake()).expect("flake should canonicalize")
    );
    assert_eq!(input_names(&resolution), ["nixpkgs", "unstable"]);
    let nixpkgs = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "nixpkgs")
        .expect("nixpkgs should be selected");
    let unstable = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "unstable")
        .expect("unstable should be selected");
    assert!(nixpkgs.changed);
    assert!(!unstable.changed);
}

#[test]
fn all_selection_omits_input_names_and_lists_every_root() {
    let mut harness = Harness::new("argv-all", "locks/before.lock");
    harness.cli.all = true;
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));

    let resolution = harness.resolve().expect("resolve should succeed");
    let args = harness.first_invocation();
    let temp = temp_lock(&args);

    assert_eq!(args, update_args(&[], harness.flake(), &temp));
    assert_eq!(
        input_names(&resolution),
        ["local", "nixpkgs", "stable", "unstable"]
    );
}

#[test]
fn changed_compares_the_whole_locked_object_and_omits_missing_fields() {
    let mut harness = Harness::new("changed", "locks/before.lock");
    harness.cli.all = true;
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));

    let resolution = harness.resolve().expect("resolve should succeed");
    let local = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "local")
        .expect("local input should be reported");
    let local_json = serde_json::to_value(local).expect("input should serialize");
    assert_eq!(
        local_json.get("before").and_then(|value| value.get("rev")),
        None
    );
    assert_eq!(
        local_json
            .get("before")
            .and_then(|value| value.get("narHash"))
            .and_then(|value| value.as_str()),
        Some("sha256-local")
    );
    assert!(!local.changed);

    let stable = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "stable")
        .expect("stable input should be reported");
    assert!(stable.changed, "an extra locked field is a change");
    assert_eq!(stable.before.rev, stable.after.rev);
    assert_eq!(stable.before.last_modified, stable.after.last_modified);
    assert_eq!(stable.before.nar_hash, stable.after.nar_hash);

    let unstable = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "unstable")
        .expect("unstable input should be reported");
    assert!(!unstable.changed);

    let nixpkgs = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "nixpkgs")
        .expect("nixpkgs input should be reported");
    assert!(nixpkgs.changed);
    assert_ne!(nixpkgs.before.rev, nixpkgs.after.rev);
}

#[test]
fn lock_file_uses_the_proposed_lock_without_calling_nix() {
    let mut harness = Harness::new("lock-file", "locks/before.lock");
    let proposed = fixture("locks/after.lock");
    let proposed_bytes = fs::read(&proposed).expect("proposed lock should be readable");
    harness.cli.lock_file = Some(proposed.clone());

    let resolution = harness.resolve().expect("resolve should succeed");

    assert!(harness.invocations().is_empty());
    assert_eq!(resolution.after_lock(), proposed);
    assert_eq!(
        fs::read(&proposed).expect("proposed lock should stay readable"),
        proposed_bytes
    );
    assert_eq!(
        input_names(&resolution),
        ["local", "nixpkgs", "stable", "unstable"]
    );
}

#[test]
fn lock_file_rejects_input_selection() {
    let mut harness = Harness::new("lock-file-input", "locks/before.lock");
    harness.cli.lock_file = Some(fixture("locks/after.lock"));
    harness.cli.input = vec!["nixpkgs".to_owned()];

    let error = harness
        .resolve()
        .expect_err("conflicting selection must fail");

    assert!(error.to_string().contains("--lock-file cannot be used"));
    assert!(harness.invocations().is_empty());
}

#[test]
fn keep_lock_rejects_lock_file() {
    let mut harness = Harness::new("keep-lock-conflict", "locks/before.lock");
    harness.cli.lock_file = Some(fixture("locks/after.lock"));
    harness.cli.keep_lock = Some(harness.dir.path().join("kept.lock"));

    let error = harness
        .resolve()
        .expect_err("conflicting options must fail");

    assert!(error.to_string().contains("--keep-lock cannot be used"));
    assert!(harness.invocations().is_empty());
}

#[test]
fn unknown_input_is_rejected_before_nix_runs() {
    let mut harness = Harness::new("unknown-input", "locks/before.lock");
    harness.cli.input = vec!["missing".to_owned()];

    let error = harness.resolve().expect_err("unknown input must fail");

    assert!(error.to_string().contains("not a root input"));
    assert!(harness.invocations().is_empty());
}

#[test]
fn input_selection_skips_unselected_follows_roots() {
    let mut harness = Harness::new("follows-unselected", "locks/follows-before.lock");
    harness.cli.input = vec!["normal".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/follows-after.lock")));

    let resolution = harness
        .resolve()
        .expect("an unselected follows root must not be resolved");

    assert_eq!(input_names(&resolution), ["normal"]);
    let normal = resolution.inputs().first().expect("normal input");
    assert!(normal.changed);
}

#[test]
fn all_selection_resolves_follows_roots() {
    let mut harness = Harness::new("follows-all", "locks/follows-before.lock");
    harness.cli.all = true;
    harness.respond(1, &Response::success(&fixture("locks/follows-after.lock")));

    let resolution = harness
        .resolve()
        .expect("a follows root should resolve to its target");
    let alias = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "alias")
        .expect("alias input should be reported");
    let normal = resolution
        .inputs()
        .iter()
        .find(|input| input.name == "normal")
        .expect("normal input should be reported");

    assert_eq!(input_names(&resolution), ["alias", "normal"]);
    assert_eq!(alias.before.rev, normal.before.rev);
    assert_eq!(alias.after.rev, normal.after.rev);
    assert_eq!(
        alias.before.rev.as_deref(),
        Some("1111111111111111111111111111111111111111")
    );
    assert_eq!(
        alias.after.rev.as_deref(),
        Some("2222222222222222222222222222222222222222")
    );
    assert!(alias.changed);
    assert!(normal.changed);
}

#[test]
fn missing_follow_target_is_a_clear_error() {
    let mut harness = Harness::new("follows-missing", "locks/follows.lock");
    harness.cli.all = true;

    let error = harness
        .resolve()
        .expect_err("a missing follow target must fail");
    let message = error.to_string();

    assert!(message.contains("follows"));
    assert!(message.contains("nixpkgs"));
    assert!(harness.invocations().is_empty());
}

#[test]
fn follows_cycle_is_a_clear_error() {
    let mut harness = Harness::new("follows-cycle", "locks/follows-cycle.lock");
    harness.cli.all = true;

    let error = harness.resolve().expect_err("a follows cycle must fail");

    assert!(error.to_string().contains("cycle"));
    assert!(harness.invocations().is_empty());
}

#[test]
fn empty_root_selects_zero_inputs() {
    let mut harness = Harness::new("empty-root", "locks/empty.lock");
    harness.cli.all = true;
    harness.respond(1, &Response::success(&fixture("locks/empty.lock")));

    let resolution = harness.resolve().expect("an empty root should succeed");
    let args = harness.first_invocation();
    let temp = temp_lock(&args);

    assert!(resolution.inputs().is_empty());
    assert_eq!(args, update_args(&[], harness.flake(), &temp));
}

#[test]
fn flake_validation_rejects_bad_flakes_before_nix_runs() {
    let harness = Harness::new("flake-validation", "locks/before.lock");
    let empty = harness.dir.path().join("empty");
    fs::create_dir_all(&empty).expect("empty directory should be creatable");
    let file = harness.dir.path().join("not-a-directory");
    fs::write(&file, "not a directory").expect("file should be writable");
    let cases = vec![
        (PathBuf::from("github:NixOS/nixpkgs"), "local"),
        (harness.dir.path().join("absent"), "does not exist"),
        (file, "not a directory"),
        (empty, "no flake.lock"),
    ];

    for (flake, expected) in cases {
        let mut cli = cli_for(&flake);
        cli.input = vec!["nixpkgs".to_owned()];
        let error = lock::resolve(&cli, &harness.nix()).expect_err("bad flake must fail");
        assert!(
            error.to_string().contains(expected),
            "unexpected error: {error}"
        );
    }
    assert!(harness.invocations().is_empty());
}

#[test]
fn a_failed_update_reports_argv_status_and_stderr() {
    let mut harness = Harness::new("failure", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.respond(1, &Response::failure("error: unable to update the lock"));
    let lock_path = harness.flake().join("flake.lock");
    let bytes = fs::read(&lock_path).expect("lock should be readable");
    let mtime = pin_mtime(&lock_path);

    let error = harness.resolve().expect_err("failing update must fail");
    let message = error.to_string();
    let args = harness.first_invocation();
    let temp = temp_lock(&args);

    assert_eq!(args.first().map(String::as_str), Some("flake"));
    assert_eq!(args.get(1).map(String::as_str), Some("update"));
    assert!(message.contains(&temp.display().to_string()));
    assert!(message.contains("exit status"));
    assert!(message.contains("unable to update the lock"));
    assert!(!temp.exists());
    assert_untouched(&lock_path, &bytes, mtime);
}

#[test]
fn a_successful_run_keeps_the_before_lock_bytes_and_mtime() {
    let mut harness = Harness::new("success-clean", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
    let lock_path = harness.flake().join("flake.lock");
    let bytes = fs::read(&lock_path).expect("lock should be readable");
    let mtime = pin_mtime(&lock_path);

    let resolution = harness.resolve().expect("resolve should succeed");
    let temp = temp_lock(&harness.first_invocation());

    assert_untouched(&lock_path, &bytes, mtime);
    assert!(temp.exists(), "the temp lock lives until the run ends");
    drop(resolution);
    assert!(!temp.exists(), "the temp lock is removed after the run");
}

#[test]
fn keep_lock_creates_parents_and_removes_the_temp() {
    let mut harness = Harness::new("keep-lock", "locks/before.lock");
    let kept = harness.dir.path().join("nested/deeper/kept.lock");
    prepare_keep_lock(&mut harness, &kept);

    let resolution = harness.resolve().expect("resolve should succeed");
    assert_kept_lock(&kept);
    let temp = temp_lock(&harness.first_invocation());
    drop(resolution);
    assert!(!temp.exists());
    assert!(kept.exists());
}

#[test]
fn keep_lock_overwrites_an_existing_file() {
    let mut harness = Harness::new("keep-lock-overwrite", "locks/before.lock");
    let kept = harness.dir.path().join("kept.lock");
    fs::write(&kept, "old contents").expect("old lock should be writable");
    prepare_keep_lock(&mut harness, &kept);

    let resolution = harness.resolve().expect("resolve should succeed");
    assert_kept_lock(&kept);
    drop(resolution);
}

#[test]
fn two_runs_get_distinct_temp_locks() {
    let mut harness = Harness::new("unique-temp", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
    harness.respond(2, &Response::success(&fixture("locks/after.lock")));

    let first = harness.resolve().expect("first resolve should succeed");
    let second = harness.resolve().expect("second resolve should succeed");
    let records = harness.invocations();

    let first_temp = temp_lock(records.first().expect("first invocation"));
    let second_temp = temp_lock(records.get(1).expect("second invocation"));
    assert_ne!(first_temp, second_temp);
    drop((first, second));
    assert!(!first_temp.exists());
    assert!(!second_temp.exists());
}

#[test]
fn missing_binary_is_a_clear_error() {
    let error =
        Nix::new("./definitely-not-a-nix-binary").expect_err("an explicit missing path must fail");

    assert!(error.to_string().contains("definitely-not-a-nix-binary"));
}

#[test]
fn bare_missing_binary_is_a_clear_error_at_spawn_time() {
    let nix = Nix::new("nix-bump-preview-missing-binary").expect("bare names stay a PATH lookup");

    let error = nix
        .run(&diff_command())
        .expect_err("a missing binary must fail");

    assert!(
        error
            .to_string()
            .contains("nix-bump-preview-missing-binary")
    );
}

#[test]
fn relative_nix_bin_is_resolved_against_the_current_directory() {
    let mut harness = Harness::new("relative-bin", "locks/before.lock");
    harness.cli.input = vec!["nixpkgs".to_owned()];
    harness.respond(1, &Response::success(&fixture("locks/after.lock")));
    let nix = harness.nix_with_bin(Path::new("tests/fixtures/fake-nix"));

    lock::resolve(&harness.cli, &nix).expect("relative nix bin should run");

    assert_eq!(harness.invocations().len(), 1);
}
