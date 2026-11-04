#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Harden Nix invocation, errors, and cleanup",
  tags: ("nbp", "cli", "lock", "eval", "tests"),
  priority: 7,
  difficulty: 3,
  kind: "refactor",
  assignee: "agent",
  estimate: (src-files: 2),
  status: done(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("parse-the-build-and-fetch-cost.typ"),
)

== Summary

Harden the runner and the error paths. `src/nix.rs` already exists from the
lock task and every command task uses it; this task does not create or
redesign the runner. It makes failures readable, proves cleanup, and checks
that every recorded command is one of the safe forms.

Source of truth: `docs/design.md` section Safety invariants; `docs/report.md` section Exit codes.

== Goals

- Product: `no-build`, `proposed-lock`, `honest`.
- Process: `test-first`, `nix-only`, `focused`.

== Acceptance criteria

- A failed Nix call reports the argv, the exit status, and Nix's stderr, and
  exits 2. No library path uses `unwrap` or `expect`.
- The temporary proposed lock is removed on success and on failure.
  `--keep-lock` copies it first. `--lock-file` is never modified. A test proves
  no temp path survives either path.
- Missing `--nix-bin`/`NIX_BIN` executable is a clear error, not a panic.
  The precedence stays as task 2 defined it; this task only tests it.
- Tests cover a failed update, a failed eval, a failed dry-run, a missing Nix
  binary, a successful cleanup, and a failed cleanup path.
- A fake-Nix test asserts every recorded command is one of the safe forms in
  `docs/design.md`: no `nix build` without `--dry-run`, no
  `nix store diff-closures` without `--derivation`, and no activation command.
- `tests/it/main.rs` declares `mod errors;`.
- Run `just outdatty-update` and commit the refreshed `outdatty.lock`.

== Non-goals

- New user-facing features.
- Retrying, timeouts, parallel execution, or another runner design.
- Changing the CLI contract or the report schema.

== Files

- `src/nix.rs`
- `src/main.rs`
- `src/lock.rs`
- `src/eval.rs`
- `tests/it/errors.rs`
- `tests/it/main.rs`
- `outdatty.lock`
