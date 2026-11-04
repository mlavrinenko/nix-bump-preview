use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use thiserror::Error;

/// A typed Nix invocation. The program is supplied by [`Nix`], so no caller
/// builds a raw argv or a [`Command`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixCommand {
    args: Vec<OsString>,
}

impl NixCommand {
    fn from_args(args: Vec<OsString>) -> Self {
        Self { args }
    }

    /// `nix flake update <input>... --flake <flake> --output-lock-file <tmp>`.
    ///
    /// An empty `inputs` slice produces the `--all` form with no input names.
    #[must_use]
    pub fn flake_update(flake: &Path, inputs: &[String], output_lock: &Path) -> Self {
        let mut args = vec![OsString::from("flake"), OsString::from("update")];
        args.extend(inputs.iter().map(|input| OsString::from(input.as_str())));
        args.push(OsString::from("--flake"));
        args.push(flake.as_os_str().to_owned());
        args.push(OsString::from("--output-lock-file"));
        args.push(output_lock.as_os_str().to_owned());
        Self::from_args(args)
    }

    /// `nix path-info --derivation --no-write-lock-file [--reference-lock-file <lock>] <flake>#<installable>`.
    #[must_use]
    pub fn path_info(flake: &Path, installable: &str, reference_lock: Option<&Path>) -> Self {
        let mut args = vec![
            OsString::from("path-info"),
            OsString::from("--derivation"),
            OsString::from("--no-write-lock-file"),
        ];
        push_reference_lock(&mut args, reference_lock);
        args.push(installable_spec(flake, installable));
        Self::from_args(args)
    }

    /// `nix store diff-closures --derivation <before> <after>`.
    #[must_use]
    pub fn diff_closures(before_drv: &Path, after_drv: &Path) -> Self {
        Self::from_args(vec![
            OsString::from("store"),
            OsString::from("diff-closures"),
            OsString::from("--derivation"),
            before_drv.as_os_str().to_owned(),
            after_drv.as_os_str().to_owned(),
        ])
    }

    /// `nix build --dry-run --no-write-lock-file --reference-lock-file <lock> <flake>#<installable>`.
    #[must_use]
    pub fn build_dry_run(flake: &Path, installable: &str, reference_lock: &Path) -> Self {
        let mut args = vec![
            OsString::from("build"),
            OsString::from("--dry-run"),
            OsString::from("--no-write-lock-file"),
        ];
        push_reference_lock(&mut args, Some(reference_lock));
        args.push(installable_spec(flake, installable));
        Self::from_args(args)
    }

    /// `nix eval --raw --no-write-lock-file [--reference-lock-file <lock>] <flake>#<attr>.version`.
    #[must_use]
    pub fn eval_version(flake: &Path, attr: &str, reference_lock: Option<&Path>) -> Self {
        let mut args = vec![
            OsString::from("eval"),
            OsString::from("--raw"),
            OsString::from("--no-write-lock-file"),
        ];
        push_reference_lock(&mut args, reference_lock);
        let mut spec = installable_spec(flake, attr);
        spec.push(".version");
        args.push(spec);
        Self::from_args(args)
    }

    /// The arguments after the Nix program name.
    #[must_use]
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    fn render(&self, bin: &Path) -> String {
        let mut parts = vec![bin.to_string_lossy().into_owned()];
        parts.extend(
            self.args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned()),
        );
        parts.join(" ")
    }
}

fn installable_spec(flake: &Path, attr: &str) -> OsString {
    let mut spec = OsString::from(flake.as_os_str());
    spec.push("#");
    spec.push(attr);
    spec
}

fn push_reference_lock(args: &mut Vec<OsString>, reference_lock: Option<&Path>) {
    if let Some(lock) = reference_lock {
        args.push(OsString::from("--reference-lock-file"));
        args.push(lock.as_os_str().to_owned());
    }
}

/// Captured stdout and stderr of a successful Nix call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixOutput {
    /// Standard output as UTF-8, lossily decoded.
    pub stdout: String,
    /// Standard error as UTF-8, lossily decoded.
    pub stderr: String,
}

/// A Nix executable together with any extra environment for its children.
#[derive(Debug, Clone)]
pub struct Nix {
    bin: PathBuf,
    env: Vec<(OsString, OsString)>,
}

impl Nix {
    /// Resolve `bin` for spawning.
    ///
    /// A bare name such as `nix` stays a `PATH` lookup; a relative path with a
    /// directory component is resolved against the current directory.
    ///
    /// # Errors
    ///
    /// Returns [`NixError::MissingBinary`] for an explicit path that does not
    /// exist and [`NixError::CurrentDir`] when the current directory is
    /// unreadable.
    pub fn new(bin: impl Into<PathBuf>) -> Result<Self, NixError> {
        let bin = bin.into();
        let resolved = resolve_bin(&bin)?;
        if has_explicit_path(&bin) && !resolved.is_file() {
            return Err(NixError::MissingBinary {
                path: resolved.to_string_lossy().into_owned(),
            });
        }
        Ok(Self {
            bin: resolved,
            env: Vec::new(),
        })
    }

    /// Add an environment variable for every child invocation.
    #[must_use]
    pub fn with_env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// The resolved executable path, or the bare name left for `PATH`.
    #[must_use]
    pub fn bin(&self) -> &Path {
        &self.bin
    }

    /// Run `command` and capture its output.
    ///
    /// # Errors
    ///
    /// Returns [`NixError::Spawn`] when the executable cannot be started and
    /// [`NixError::Failed`] with the full argv, exit status, and stderr when
    /// Nix exits non-zero.
    pub fn run(&self, command: &NixCommand) -> Result<NixOutput, NixError> {
        let mut process = Command::new(&self.bin);
        process.args(&command.args);
        for (key, value) in &self.env {
            process.env(key, value);
        }
        let output = process.output().map_err(|source| NixError::Spawn {
            command: command.render(&self.bin),
            source,
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !output.status.success() {
            return Err(NixError::Failed {
                command: command.render(&self.bin),
                status: output.status,
                stderr,
            });
        }
        Ok(NixOutput { stdout, stderr })
    }
}

fn has_explicit_path(bin: &Path) -> bool {
    bin.is_absolute() || bin.components().count() > 1
}

fn resolve_bin(bin: &Path) -> Result<PathBuf, NixError> {
    if !has_explicit_path(bin) {
        return Ok(bin.to_path_buf());
    }
    if bin.is_absolute() {
        return Ok(bin.to_path_buf());
    }
    env::current_dir()
        .map(|cwd| cwd.join(bin))
        .map_err(|source| NixError::CurrentDir { source })
}

/// A failure to resolve or run the Nix executable.
#[derive(Debug, Error)]
pub enum NixError {
    /// The current directory could not be read while resolving a relative path.
    #[error("cannot resolve the current directory: {source}")]
    CurrentDir {
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// An explicit executable path does not exist.
    #[error("nix executable not found: {path}")]
    MissingBinary {
        /// The resolved path that was checked.
        path: String,
    },

    /// The executable could not be started.
    #[error("failed to run `{command}`: {source}")]
    Spawn {
        /// The full argv that was attempted.
        command: String,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// The executable ran and exited non-zero.
    #[error("`{command}` failed with {status}; nix stderr:\n{stderr}")]
    Failed {
        /// The full argv that failed.
        command: String,
        /// Process exit status.
        status: ExitStatus,
        /// Everything Nix wrote to stderr.
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flake() -> PathBuf {
        PathBuf::from("/tmp/example")
    }

    fn texts(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn flake_update_names_inputs_before_the_lock_flags() {
        let command = NixCommand::flake_update(
            &flake(),
            &["nixpkgs".to_owned(), "unstable".to_owned()],
            Path::new("/tmp/proposed.lock"),
        );

        assert_eq!(
            texts(command.args()),
            [
                "flake",
                "update",
                "nixpkgs",
                "unstable",
                "--flake",
                "/tmp/example",
                "--output-lock-file",
                "/tmp/proposed.lock",
            ]
        );
    }

    #[test]
    fn flake_update_without_inputs_is_the_all_form() {
        let command = NixCommand::flake_update(&flake(), &[], Path::new("/tmp/proposed.lock"));

        assert_eq!(
            texts(command.args()),
            [
                "flake",
                "update",
                "--flake",
                "/tmp/example",
                "--output-lock-file",
                "/tmp/proposed.lock",
            ]
        );
    }

    #[test]
    fn path_info_carries_derivation_and_no_write_lock() {
        let command = NixCommand::path_info(&flake(), "example", None);

        assert_eq!(
            texts(command.args()),
            [
                "path-info",
                "--derivation",
                "--no-write-lock-file",
                "/tmp/example#example",
            ]
        );
    }

    #[test]
    fn path_info_adds_a_reference_lock_before_the_installable() {
        let command =
            NixCommand::path_info(&flake(), "example", Some(Path::new("/tmp/after.lock")));

        assert_eq!(
            texts(command.args()),
            [
                "path-info",
                "--derivation",
                "--no-write-lock-file",
                "--reference-lock-file",
                "/tmp/after.lock",
                "/tmp/example#example",
            ]
        );
    }

    #[test]
    fn diff_closures_always_carries_derivation() {
        let command = NixCommand::diff_closures(
            Path::new("/nix/store/old.drv"),
            Path::new("/nix/store/new.drv"),
        );

        assert_eq!(
            texts(command.args()),
            [
                "store",
                "diff-closures",
                "--derivation",
                "/nix/store/old.drv",
                "/nix/store/new.drv",
            ]
        );
    }

    #[test]
    fn build_dry_run_never_realizes_outputs() {
        let command = NixCommand::build_dry_run(&flake(), "example", Path::new("/tmp/after.lock"));

        assert_eq!(
            texts(command.args()),
            [
                "build",
                "--dry-run",
                "--no-write-lock-file",
                "--reference-lock-file",
                "/tmp/after.lock",
                "/tmp/example#example",
            ]
        );
    }

    #[test]
    fn eval_version_appends_the_version_suffix() {
        let command = NixCommand::eval_version(&flake(), "packages.example", None);

        assert_eq!(
            texts(command.args()),
            [
                "eval",
                "--raw",
                "--no-write-lock-file",
                "/tmp/example#packages.example.version",
            ]
        );
    }

    #[test]
    fn eval_version_adds_a_reference_lock_before_the_attr() {
        let command = NixCommand::eval_version(
            &flake(),
            "packages.example",
            Some(Path::new("/tmp/after.lock")),
        );

        assert_eq!(
            texts(command.args()),
            [
                "eval",
                "--raw",
                "--no-write-lock-file",
                "--reference-lock-file",
                "/tmp/after.lock",
                "/tmp/example#packages.example.version",
            ]
        );
    }

    #[test]
    fn a_bare_name_stays_a_path_lookup() {
        let nix = Nix::new("nix").expect("a bare name needs no filesystem check");

        assert_eq!(nix.bin(), Path::new("nix"));
    }

    #[test]
    fn a_relative_explicit_path_is_made_absolute() {
        let nix = Nix::new("tests/fixtures/fake-nix").expect("the fixture should exist");

        assert!(nix.bin().is_absolute());
        assert!(nix.bin().ends_with("tests/fixtures/fake-nix"));
    }
}
