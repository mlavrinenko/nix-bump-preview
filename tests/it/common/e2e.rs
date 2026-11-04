use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

use super::{Harness, Response, assert_untouched, pin_mtime};

pub struct FlakeSpec {
    pub name: &'static str,
    pub changed: &'static str,
    pub unchanged: &'static str,
    pub installable: &'static str,
    pub attr: &'static str,
}

pub const ALPHA: FlakeSpec = FlakeSpec {
    name: "alpha",
    changed: "alpha-cache",
    unchanged: "alpha-stable",
    installable: "packages.example-alpha",
    attr: "packages.example-alpha",
};

pub const BETA: FlakeSpec = FlakeSpec {
    name: "beta",
    changed: "beta-cache",
    unchanged: "beta-pinned",
    installable: "devShells.example-beta",
    attr: "devShells.example-beta",
};

pub fn harness_for(spec: &FlakeSpec) -> Harness {
    Harness::from_flake(spec.name, &format!("flakes/{}", spec.name))
}

pub fn drv(spec: &FlakeSpec, side: &str) -> String {
    format!("/nix/store/{}-{side}.drv\n", spec.name)
}

pub fn respond_full(harness: &Harness, spec: &FlakeSpec) {
    let proposed = harness.flake().join("proposed.lock");
    harness.respond(1, &Response::success(&proposed));
    harness.respond_with("path-info", 1, &Response::stdout(&drv(spec, "before")));
    harness.respond_with("path-info", 2, &Response::stdout(&drv(spec, "after")));
    harness.respond_with(
        "store-diff-closures",
        1,
        &Response::stdout(&format!("{}-pkg: 1.0 -> 2.0\n", spec.name)),
    );
    harness.respond_with(
        "build",
        1,
        &Response::stderr(
            "these 2 derivations will be built:\n\
             these 1 paths will be fetched (1.5 KiB download, 2.0 KiB unpacked):\n",
        ),
    );
    harness.respond_with("eval", 1, &Response::stdout("1.0\n"));
    harness.respond_with("eval", 2, &Response::stdout("2.0\n"));
}

pub fn locked_command(harness: &Harness, spec: &FlakeSpec, lock: &Path) -> Command {
    let mut command = harness.command();
    command
        .arg("--flake")
        .arg(harness.flake())
        .arg("--lock-file")
        .arg(lock)
        .arg("--installable")
        .arg(spec.installable);
    command
}

pub fn pin_before_lock(harness: &Harness) -> (PathBuf, Vec<u8>, std::time::SystemTime) {
    let lock = harness.flake().join("flake.lock");
    let bytes = fs::read(&lock).expect("before lock should be readable");
    let mtime = pin_mtime(&lock);
    (lock, bytes, mtime)
}

pub fn run_locked(spec: &FlakeSpec, format: &str) -> (Harness, String) {
    let harness = harness_for(spec);
    let (lock, bytes, mtime) = pin_before_lock(&harness);
    respond_full(&harness, spec);
    let proposed = harness.flake().join("proposed.lock");
    let assert = locked_command(&harness, spec, &proposed)
        .arg("--format")
        .arg(format)
        .assert()
        .success();
    assert_untouched(&lock, &bytes, mtime);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is UTF-8");
    (harness, stdout)
}
