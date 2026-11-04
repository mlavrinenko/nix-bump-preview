#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Add end-to-end tests with a fake Nix",
  tags: ("nbp", "tests", "cli", "json"),
  priority: 8,
  difficulty: 4,
  kind: "test",
  assignee: "agent",
  estimate: (src-files: 3),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("render-text-and-json-reports.typ"),
)

== Summary

Add the end-to-end suite. A fake `nix` executable records argv and prints
canned output for update, path-info, diff-closures, build dry-run, and eval.
The suite runs the real binary through `assert_cmd` and proves the whole
preview without network or a real store.

Source of truth: `docs/report.md` section Testing.

== Goals

- Product: `proposed-lock`, `package-delta`, `build-cost`, `no-build`,
  `machine-readable`, `generic`.
- Process: `test-first`, `parsers-tested`, `nix-only`.

== Acceptance criteria

- `tests/fixtures/fake-nix` is a portable script that logs argv to a file and
  emits deterministic stdout/stderr for every call the tool makes. It is
  POSIX shell, not bash-only, so the Windows release job does not depend on it.
- Fixture locks cover a changed input, an unchanged input, `--all`, and
  `--lock-file`.
- Two unrelated fixture flakes and installables prove `generic`. Neither uses
  a real hostname, system name, or personal path.
- E2E tests cover text, JSON, `--lock-file`, `--keep-lock`, `--no-delta`,
  `--no-cost`, `--exit-code`, and the failure exits.
- A full success and a full failure both assert that the before `flake.lock`
  keeps its bytes and mtime.
- The fake records every argv. A test asserts no `nix build` without
  `--dry-run`, no `nix store diff-closures` without `--derivation`, no
  activation command, and that `--no-delta`/`--no-cost` omit their calls.
- `just test` runs the suite offline. A real-Nix test is `#[ignore]` and runs
  only when `NIX_BMP_REAL_FLAKE` points at a local fixture flake.
- `just check` is green.
- Run `just outdatty-update` and commit the refreshed `outdatty.lock`.

== Non-goals

- Testing Nix itself or requiring network access in the gate.
- A second test binary; add files under `tests/it/` and `mod` them in
  `tests/it/main.rs`.

== Files

- `tests/fixtures/fake-nix`
- `tests/fixtures/flakes/`
- `tests/fixtures/locks/`
- `tests/fixtures/captured/`
- `tests/it/e2e.rs`
- `tests/it/main.rs`
- `outdatty.lock`
