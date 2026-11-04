use assert_cmd::Command;
use predicates::prelude::*;

fn binary() -> Command {
    Command::cargo_bin(env!("CARGO_PKG_NAME")).expect("binary should build")
}

#[test]
fn reports_version() {
    binary()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_every_visible_option() {
    let assert = binary().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    for option in [
        "-f, --flake",
        "-i, --input",
        "--all",
        "-l, --lock-file",
        "--installable",
        "--attr",
        "--format",
        "--keep-lock",
        "--no-delta",
        "--no-cost",
        "--exit-code",
        "-h, --help",
        "-V, --version",
    ] {
        assert!(stdout.contains(option), "help is missing {option}");
    }

    assert!(!stdout.contains("--nix-bin"), "help shows hidden --nix-bin");
    assert!(
        !stdout.contains("--print-schema"),
        "help shows hidden --print-schema"
    );
}

fn assert_usage_error(args: &[&str], expected: &str) {
    binary()
        .args(args)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(expected));
}

#[test]
fn missing_installable_is_a_usage_error() {
    assert_usage_error(
        &["--input", "unstable"],
        "at least one --installable is required",
    );
}

#[test]
fn missing_input_or_all_is_a_usage_error() {
    assert_usage_error(
        &[
            "--installable",
            "nixosConfigurations.example.config.system.build.toplevel",
        ],
        "either --input or --all is required unless --lock-file is given",
    );
}

#[test]
fn input_and_all_conflict() {
    assert_usage_error(
        &[
            "--input",
            "unstable",
            "--all",
            "--installable",
            "nixosConfigurations.example.config.system.build.toplevel",
        ],
        "--input and --all cannot be used together",
    );
}

#[test]
fn lock_file_rejects_input() {
    assert_usage_error(
        &[
            "--lock-file",
            "proposed.lock",
            "--input",
            "unstable",
            "--installable",
            "nixosConfigurations.example.config.system.build.toplevel",
        ],
        "--lock-file cannot be used with --input or --all",
    );
}

#[test]
fn lock_file_rejects_all() {
    assert_usage_error(
        &[
            "--lock-file",
            "proposed.lock",
            "--all",
            "--installable",
            "nixosConfigurations.example.config.system.build.toplevel",
        ],
        "--lock-file cannot be used with --input or --all",
    );
}

#[test]
fn keep_lock_rejects_lock_file() {
    assert_usage_error(
        &[
            "--lock-file",
            "proposed.lock",
            "--keep-lock",
            "kept.lock",
            "--installable",
            "nixosConfigurations.example.config.system.build.toplevel",
        ],
        "--keep-lock cannot be used with --lock-file",
    );
}

#[test]
fn valid_invocation_reports_the_missing_pipeline() {
    binary()
        .args([
            "--lock-file",
            "proposed.lock",
            "--installable",
            "nixosConfigurations.example.config.system.build.toplevel",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "error: preview pipeline is not implemented yet",
        ));
}

#[test]
fn print_schema_alone_succeeds() {
    binary()
        .arg("--print-schema")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn print_schema_matches_the_committed_file() {
    let assert = binary().arg("--print-schema").assert().success();
    let printed = &assert.get_output().stdout;
    let committed = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schema/report.schema.json"
    ))
    .expect("schema/report.schema.json should be committed");

    assert_eq!(
        printed.as_slice(),
        committed.as_slice(),
        "printed schema differs from schema/report.schema.json; run `just schema`"
    );
}

#[test]
fn print_schema_wins_over_validation() {
    binary()
        .args(["--print-schema", "--input", "unstable", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}
