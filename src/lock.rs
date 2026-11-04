use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value};
use thiserror::Error;

use crate::cli::{Cli, CliError};
use crate::nix::{Nix, NixCommand, NixError};
use crate::report::{InputReport, Locked};

const TEMP_ATTEMPTS: u64 = 128;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The resolved before and proposed lock entries for the selected inputs.
#[derive(Debug)]
pub struct Resolution {
    flake: PathBuf,
    inputs: Vec<InputReport>,
    after_lock: AfterLock,
}

impl Resolution {
    /// Canonical path to the flake directory.
    #[must_use]
    pub fn flake(&self) -> &Path {
        &self.flake
    }

    /// Selected inputs sorted by name.
    #[must_use]
    pub fn inputs(&self) -> &[InputReport] {
        &self.inputs
    }

    /// Path to the proposed lock.
    ///
    /// For a generated lock this is a temporary file removed when the
    /// resolution is dropped; for `--lock-file` it is the caller's path.
    #[must_use]
    pub fn after_lock(&self) -> &Path {
        self.after_lock.path()
    }
}

/// Read the before lock and resolve the proposed lock for `cli`.
///
/// No `flake.lock` is written. When `--lock-file` is absent the proposed lock
/// is written to a unique temporary file that lives until the returned
/// [`Resolution`] is dropped.
///
/// # Errors
///
/// Returns a [`LockError`] for an invalid selection, a non-local or incomplete
/// flake, a malformed lock, an unknown input, or a failing Nix call.
pub fn resolve(cli: &Cli, nix: &Nix) -> Result<Resolution, LockError> {
    validate_selection(cli)?;
    let flake = validate_flake(&cli.flake)?;
    let before = read_lock(&flake.join("flake.lock"))?;
    let selection = select_inputs(cli, &root_names(&before)?)?;
    let before_locked = selected_inputs(&before, &selection)?;
    let after_lock = match &cli.lock_file {
        Some(path) => AfterLock::Provided(path.clone()),
        None => AfterLock::Temporary(resolve_proposed(cli, nix, &flake, &selection)?),
    };
    if let Some(destination) = &cli.keep_lock {
        copy_keep_lock(after_lock.path(), destination)?;
    }
    let after = read_lock(after_lock.path())?;
    let after_locked = selected_inputs(&after, &selection)?;
    let inputs = build_reports(&selection, &before_locked, &after_locked)?;
    Ok(Resolution {
        flake,
        inputs,
        after_lock,
    })
}

fn validate_selection(cli: &Cli) -> Result<(), LockError> {
    if cli.all && !cli.input.is_empty() {
        return Err(CliError::InputAndAll.into());
    }
    if cli.lock_file.is_some() && (cli.all || !cli.input.is_empty()) {
        return Err(CliError::LockFileWithInput.into());
    }
    if cli.lock_file.is_some() && cli.keep_lock.is_some() {
        return Err(CliError::KeepLockWithLockFile.into());
    }
    if cli.lock_file.is_none() && cli.input.is_empty() && !cli.all {
        return Err(CliError::MissingInput.into());
    }
    Ok(())
}

fn validate_flake(flake: &Path) -> Result<PathBuf, LockError> {
    let raw = flake.to_string_lossy().into_owned();
    if is_remote(&raw) {
        return Err(LockError::RemoteFlake { path: raw });
    }
    if !flake.exists() {
        return Err(LockError::MissingFlake { path: raw });
    }
    let canonical = fs::canonicalize(flake).map_err(|source| LockError::Read {
        path: raw.clone(),
        source,
    })?;
    if !canonical.is_dir() {
        return Err(LockError::NotADirectory {
            path: canonical.display().to_string(),
        });
    }
    if !canonical.join("flake.lock").is_file() {
        return Err(LockError::MissingFlakeLock {
            path: canonical.display().to_string(),
        });
    }
    Ok(canonical)
}

fn is_remote(spec: &str) -> bool {
    spec.contains("://")
        || spec.starts_with("github:")
        || spec.starts_with("gitlab:")
        || spec.starts_with("sourcehut:")
        || spec.starts_with("git+")
}

fn read_lock(path: &Path) -> Result<Value, LockError> {
    let rendered = path.display().to_string();
    let text = fs::read_to_string(path).map_err(|source| LockError::Read {
        path: rendered.clone(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| LockError::Parse {
        path: rendered,
        source,
    })
}

fn resolve_proposed(
    cli: &Cli,
    nix: &Nix,
    flake: &Path,
    selection: &[String],
) -> Result<TempLock, LockError> {
    let temp = TempLock::create()?;
    let inputs = if cli.all {
        Vec::new()
    } else {
        selection.to_vec()
    };
    nix.run(&NixCommand::flake_update(flake, &inputs, temp.path()))?;
    if !temp.path().is_file() {
        return Err(LockError::MissingProposedLock {
            path: temp.path().display().to_string(),
        });
    }
    Ok(temp)
}

fn root_names(lock: &Value) -> Result<BTreeSet<String>, LockError> {
    let inputs = root_inputs_map(lock)?;
    Ok(inputs
        .map(|inputs| inputs.keys().cloned().collect())
        .unwrap_or_default())
}

fn root_inputs_map(lock: &Value) -> Result<Option<&Map<String, Value>>, LockError> {
    if let Some(root) = lock.get("root").and_then(Value::as_str) {
        let node = lock
            .get("nodes")
            .and_then(|nodes| nodes.get(root))
            .ok_or(LockError::NoRoot)?;
        return Ok(node.get("inputs").and_then(Value::as_object));
    }
    if let Some(inputs) = lock.get("inputs").and_then(Value::as_object) {
        return Ok(Some(inputs));
    }
    Err(LockError::NoRoot)
}

fn select_inputs(cli: &Cli, roots: &BTreeSet<String>) -> Result<Vec<String>, LockError> {
    if cli.lock_file.is_some() || cli.all {
        return Ok(roots.iter().cloned().collect());
    }
    let mut names = BTreeSet::new();
    for name in &cli.input {
        if !roots.contains(name) {
            return Err(LockError::UnknownInput { name: name.clone() });
        }
        names.insert(name.clone());
    }
    Ok(names.into_iter().collect())
}

fn selected_inputs(lock: &Value, names: &[String]) -> Result<BTreeMap<String, Value>, LockError> {
    if names.is_empty() {
        return Ok(BTreeMap::new());
    }
    let Some(inputs) = root_inputs_map(lock)? else {
        return Ok(BTreeMap::new());
    };
    let mut locked = BTreeMap::new();
    for name in names {
        let mut visiting = BTreeSet::new();
        locked.insert(
            name.clone(),
            resolve_root_input(lock, inputs, name, &mut visiting)?,
        );
    }
    Ok(locked)
}

fn resolve_root_input(
    lock: &Value,
    inputs: &Map<String, Value>,
    name: &str,
    visiting: &mut BTreeSet<String>,
) -> Result<Value, LockError> {
    if !visiting.insert(name.to_owned()) {
        return Err(LockError::FollowsCycle {
            name: name.to_owned(),
        });
    }
    let result = match inputs.get(name) {
        Some(Value::String(node)) => locked_node(lock, node, name),
        Some(Value::Object(map)) => {
            if let Some(Value::String(node)) = map.get("path") {
                locked_node(lock, node, name)
            } else if let Some(Value::String(target)) = map.get("follows") {
                if inputs.contains_key(target) {
                    resolve_root_input(lock, inputs, target, visiting)
                } else {
                    Err(LockError::FollowsMissing {
                        name: name.to_owned(),
                        target: target.clone(),
                    })
                }
            } else {
                Err(LockError::UnresolvedInput {
                    name: name.to_owned(),
                })
            }
        }
        _ => Err(LockError::UnresolvedInput {
            name: name.to_owned(),
        }),
    };
    visiting.remove(name);
    result
}

fn locked_node(lock: &Value, node: &str, input: &str) -> Result<Value, LockError> {
    lock.get("nodes")
        .and_then(|nodes| nodes.get(node))
        .and_then(|node| node.get("locked"))
        .filter(|locked| locked.is_object())
        .cloned()
        .ok_or_else(|| LockError::UnresolvedInput {
            name: input.to_owned(),
        })
}

fn build_reports(
    selection: &[String],
    before: &BTreeMap<String, Value>,
    after: &BTreeMap<String, Value>,
) -> Result<Vec<InputReport>, LockError> {
    selection
        .iter()
        .map(|name| {
            let before_locked = before
                .get(name)
                .ok_or_else(|| LockError::UnknownInput { name: name.clone() })?;
            let after_locked = after
                .get(name)
                .ok_or_else(|| LockError::MissingAfterInput { name: name.clone() })?;
            Ok(InputReport {
                name: name.clone(),
                changed: before_locked != after_locked,
                before: locked_report(before_locked),
                after: locked_report(after_locked),
            })
        })
        .collect()
}

fn locked_report(locked: &Value) -> Locked {
    Locked {
        rev: string_field(locked, "rev"),
        last_modified: locked.get("lastModified").and_then(Value::as_u64),
        nar_hash: string_field(locked, "narHash"),
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn copy_keep_lock(from: &Path, to: &Path) -> Result<(), LockError> {
    if let Some(parent) = to.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|source| LockError::KeepLock {
                path: to.display().to_string(),
                source,
            })?;
        }
    }
    fs::copy(from, to).map_err(|source| LockError::KeepLock {
        path: to.display().to_string(),
        source,
    })?;
    Ok(())
}

#[derive(Debug)]
enum AfterLock {
    Provided(PathBuf),
    Temporary(TempLock),
}

impl AfterLock {
    fn path(&self) -> &Path {
        match self {
            Self::Provided(path) => path,
            Self::Temporary(temp) => temp.path(),
        }
    }
}

#[derive(Debug)]
struct TempLock {
    path: PathBuf,
}

impl TempLock {
    fn create() -> Result<Self, LockError> {
        let dir = env::temp_dir();
        for _ in 0..TEMP_ATTEMPTS {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = dir.join(format!("nix-bump-preview-{}-{counter}.lock", process::id()));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(Self { path }),
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(LockError::TempLock {
                        dir: dir.display().to_string(),
                        source,
                    });
                }
            }
        }
        Err(LockError::TempLock {
            dir: dir.display().to_string(),
            source: io::Error::new(
                io::ErrorKind::AlreadyExists,
                "no unique temporary lock name",
            ),
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// A failure while resolving the proposed lock.
#[derive(Debug, Error)]
pub enum LockError {
    /// The selection flags are inconsistent.
    #[error(transparent)]
    Cli(#[from] CliError),

    /// A Nix call failed.
    #[error(transparent)]
    Nix(#[from] NixError),

    /// The flake is a remote URL rather than a local directory.
    #[error("--flake must be a local directory, not a remote URL: {path}")]
    RemoteFlake { path: String },

    /// The flake directory does not exist.
    #[error("flake directory does not exist: {path}")]
    MissingFlake { path: String },

    /// The flake path exists but is not a directory.
    #[error("flake path is not a directory: {path}")]
    NotADirectory { path: String },

    /// The flake directory has no `flake.lock`.
    #[error("flake directory has no flake.lock: {path}")]
    MissingFlakeLock { path: String },

    /// A lock file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: io::Error,
    },

    /// A lock file is not valid JSON.
    #[error("cannot parse {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    /// The lock has no root node.
    #[error("lock file has no root node")]
    NoRoot,

    /// A root input does not point at a locked node.
    #[error("input '{name}' has no resolved locked node")]
    UnresolvedInput { name: String },

    /// A follows root names an input that does not exist.
    #[error("input '{name}' follows missing input '{target}'")]
    FollowsMissing { name: String, target: String },

    /// Follows roots form a cycle.
    #[error("input '{name}' follows a cycle")]
    FollowsCycle { name: String },

    /// `--input` named an input that is not a root input.
    #[error("input '{name}' is not a root input")]
    UnknownInput { name: String },

    /// The proposed lock lacks an input present in the before lock.
    #[error("input '{name}' is missing from the proposed lock")]
    MissingAfterInput { name: String },

    /// Nix exited successfully without writing the proposed lock.
    #[error("nix flake update did not write the proposed lock to {path}")]
    MissingProposedLock { path: String },

    /// A unique temporary lock could not be created.
    #[error("cannot create a temporary lock in {dir}: {source}")]
    TempLock {
        dir: String,
        #[source]
        source: io::Error,
    },

    /// The proposed lock could not be copied to `--keep-lock`.
    #[error("cannot copy the proposed lock to {path}: {source}")]
    KeepLock {
        path: String,
        #[source]
        source: io::Error,
    },
}

#[cfg(test)]
#[path = "lock_tests.rs"]
mod tests;
