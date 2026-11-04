use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use nix_bump_preview::cli::{Cli, OutputFormat};
use nix_bump_preview::lock;
use nix_bump_preview::nix::{Nix, NixCommand};

const DELIMITER: &str = "=== invocation ===";

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(label: &str) -> Self {
        let count = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "nix-bump-preview-test-{}-{label}-{count}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test directory should be creatable");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fake_nix() -> PathBuf {
    fixture("fake-nix")
}

pub struct Response<'a> {
    status: u8,
    stdout: &'a str,
    stderr: &'a str,
    lock: Option<&'a Path>,
}

impl<'a> Response<'a> {
    pub fn success(lock: &'a Path) -> Self {
        Self {
            status: 0,
            stdout: "",
            stderr: "",
            lock: Some(lock),
        }
    }

    pub fn failure(stderr: &'a str) -> Self {
        Self {
            status: 1,
            stdout: "",
            stderr,
            lock: None,
        }
    }

    pub fn stdout(stdout: &'a str) -> Self {
        Self {
            status: 0,
            stdout,
            stderr: "",
            lock: None,
        }
    }

    pub fn with_stderr(mut self, stderr: &'a str) -> Self {
        self.stderr = stderr;
        self
    }
}

fn write_response(scenario: &Path, key: &str, index: u32, response: &Response) {
    let dir = scenario.join(key).join(index.to_string());
    fs::create_dir_all(&dir).expect("response directory should be creatable");
    fs::write(dir.join("status"), response.status.to_string()).expect("status should be writable");
    fs::write(dir.join("stdout"), response.stdout).expect("stdout should be writable");
    fs::write(dir.join("stderr"), response.stderr).expect("stderr should be writable");
    if let Some(lock) = response.lock {
        fs::copy(lock, dir.join("lock")).expect("lock fixture should be copyable");
    }
}

fn invocations(log: &Path) -> Vec<Vec<String>> {
    let Ok(text) = fs::read_to_string(log) else {
        return Vec::new();
    };
    text.split(&format!("{DELIMITER}\n"))
        .filter(|block| !block.is_empty())
        .map(|block| block.lines().map(ToOwned::to_owned).collect())
        .collect()
}

pub fn temp_lock(args: &[String]) -> PathBuf {
    let index = args
        .iter()
        .position(|arg| arg == "--output-lock-file")
        .expect("argv should carry --output-lock-file");
    args.get(index + 1)
        .map(PathBuf::from)
        .expect("--output-lock-file should have a value")
}

pub fn cli_for(flake: &Path) -> Cli {
    Cli {
        flake: flake.to_path_buf(),
        input: Vec::new(),
        all: false,
        lock_file: None,
        installable: vec!["nixosConfigurations.example.config.system.build.toplevel".to_owned()],
        attr: Vec::new(),
        format: OutputFormat::Text,
        keep_lock: None,
        no_delta: false,
        no_cost: false,
        exit_code: false,
        nix_bin: fake_nix(),
        print_schema: false,
    }
}

pub struct Harness {
    pub dir: TestDir,
    pub cli: Cli,
    flake: PathBuf,
    scenario: PathBuf,
    log: PathBuf,
}

impl Harness {
    pub fn new(label: &str, before_fixture: &str) -> Self {
        let dir = TestDir::new(label);
        let flake = dir.path().join("flake");
        fs::create_dir_all(&flake).expect("flake directory should be creatable");
        fs::copy(fixture(before_fixture), flake.join("flake.lock"))
            .expect("before lock fixture should be copyable");
        let scenario = dir.path().join("scenario");
        let log = dir.path().join("argv.log");
        let cli = cli_for(&flake);
        Self {
            dir,
            cli,
            flake,
            scenario,
            log,
        }
    }

    pub fn flake(&self) -> &Path {
        &self.flake
    }

    pub fn respond(&self, index: u32, response: &Response) {
        self.respond_with("flake-update", index, response);
    }

    pub fn respond_with(&self, key: &str, index: u32, response: &Response) {
        write_response(&self.scenario, key, index, response);
    }

    pub fn nix(&self) -> Nix {
        self.nix_with_bin(&fake_nix())
    }

    pub fn nix_with_bin(&self, bin: &Path) -> Nix {
        Nix::new(bin)
            .expect("fake nix should resolve")
            .with_env("FAKE_NIX_SCENARIO", self.scenario.as_os_str())
            .with_env("FAKE_NIX_LOG", self.log.as_os_str())
    }

    pub fn resolve(&self) -> Result<lock::Resolution, lock::LockError> {
        lock::resolve(&self.cli, &self.nix())
    }

    pub fn invocations(&self) -> Vec<Vec<String>> {
        invocations(&self.log)
    }

    pub fn first_invocation(&self) -> Vec<String> {
        self.invocations()
            .first()
            .cloned()
            .expect("nix should have been invoked")
    }
}

pub fn canonical(path: &Path) -> String {
    fs::canonicalize(path)
        .expect("path should canonicalize")
        .display()
        .to_string()
}

pub fn input_names(resolution: &lock::Resolution) -> Vec<&str> {
    resolution
        .inputs()
        .iter()
        .map(|input| input.name.as_str())
        .collect()
}

pub fn diff_command() -> NixCommand {
    NixCommand::diff_closures(Path::new("/tmp/old.drv"), Path::new("/tmp/new.drv"))
}

pub fn pin_mtime(path: &Path) -> SystemTime {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let file = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("lock should be writable");
    file.set_times(fs::FileTimes::new().set_modified(time))
        .expect("mtime should be settable");
    time
}

pub fn assert_untouched(path: &Path, bytes: &[u8], mtime: SystemTime) {
    assert_eq!(fs::read(path).expect("lock should be readable"), bytes);
    assert_eq!(
        fs::metadata(path)
            .expect("lock metadata should be readable")
            .modified()
            .expect("mtime should be available"),
        mtime
    );
}
