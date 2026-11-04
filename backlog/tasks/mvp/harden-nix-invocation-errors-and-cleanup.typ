#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Harden Nix invocation, errors, and cleanup",
  tags: ("nbp", "cli", "lock", "eval", "tests"),
  priority: 7,
  difficulty: 3,
  kind: "refactor",
  assignee: "agent",
  estimate: (src-files: 2),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on(
      "resolve-a-proposed-lock-without-writing-flake-lock.typ",
      "evaluate-old-and-proposed-derivations-for-arbitrary-installables.typ",
    ),
)

== Summary

Centralise process execution and error handling. Every Nix call goes through
one module that builds an argv array, captures stdout and stderr, and reports
the exact command on failure. Prove that temporary locks are cleaned up on
every path and that the before lock is never touched.

Source of truth: `docs/design.md` sections Safety invariants and Exit codes.


== Goals

- Product: `no-build`, `proposed-lock`, `honest`.
- Process: `test-first`, `nix-only`, `focused`.


== Acceptance criteria

- `src/nix.rs` exposes a typed runner used by every module. It never passes a
  shell string to `sh -c`.
- A failed Nix call reports the argv, the exit status, and Nix's stderr, and
  exits 2. No library path uses `unwrap` or `expect`.
- `--nix-bin` and `NIX_BIN` choose the executable. A missing binary is a
  clear error, not a panic.
- The temporary proposed lock is removed on success and on failure.
  `--keep-lock` copies it first. `--lock-file` is never modified.
- Tests cover a failed update, a failed eval, a failed dry-run, a missing Nix
  binary, and a successful cleanup. They assert no temp path survives.
- A fake-Nix test asserts every recorded command is one of the safe forms in
  `docs/design.md`.


== Non-goals

- New user-facing features.
- Retrying, timeouts, or parallel execution.


== Files

- `src/nix.rs`
- `src/main.rs`
- `src/lock.rs`
- `src/eval.rs`
- `tests/it/errors.rs`

