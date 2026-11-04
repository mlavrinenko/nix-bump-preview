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
    + depends-on(
      "render-text-and-json-reports.typ",
      "harden-nix-invocation-errors-and-cleanup.typ",
    ),
)

== Summary

Add the end-to-end suite. A fake `nix` executable records argv and prints
canned output for update, path-info, diff-closures, build dry-run, and eval.
The suite runs the real binary through `assert_cmd` and proves the whole
preview without network or a real store.

Source of truth: `docs/design.md` section Testing.


== Goals

- Product: `proposed-lock`, `package-delta`, `build-cost`, `no-build`,
  `machine-readable`.
- Process: `test-first`, `parsers-tested`, `nix-only`.


== Acceptance criteria

- `tests/fixtures/fake-nix` is a portable script that logs argv to a file and
  emits deterministic stdout/stderr for every call the tool makes.
- Fixture locks cover a changed input, an unchanged input, and `--all`.
- E2E tests cover text, JSON, `--lock-file`, `--keep-lock`, `--no-delta`,
  `--no-cost`, `--exit-code`, and the failure exits.
- The fake records every argv. A test asserts no `nix build` without
  `--dry-run`, no `nix store diff-closures` without `--derivation`, and no
  activation command.
- `just test` runs the suite offline. A real-Nix test is `#[ignore]` and runs
  only when a local fixture flake is supplied through an environment
  variable.
- `just check` is green.


== Non-goals

- Testing Nix itself or requiring network access in the gate.
- A second test binary; add files under `tests/it/` and `mod` them in
  `tests/it/main.rs`.


== Files

- `tests/fixtures/fake-nix`
- `tests/fixtures/flakes/`
- `tests/fixtures/locks/`
- `tests/it/e2e.rs`
- `tests/it/main.rs`
- `src/nix.rs`

