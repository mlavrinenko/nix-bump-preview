#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Resolve a proposed lock without writing flake.lock",
  tags: ("nbp", "lock", "cli"),
  priority: 8,
  difficulty: 4,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 3),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("implement-the-cli-contract-and-json-report-schema.typ"),
)

== Summary

Implement the lock module. It reads the before lock, resolves a proposed lock
with `nix flake update --output-lock-file`, or accepts an existing proposed
lock with `--lock-file`, and reports which selected root inputs changed. The
repository's `flake.lock` must stay byte-identical.

Source of truth: `docs/design.md` sections Options, Sequence, Nix calls, and
Safety invariants.


== Goals

- Product: `proposed-lock`, `no-build`, `generic`.
- Process: `test-first`, `nix-only`, `no-personal`.


== Acceptance criteria

- `--input <name>` is repeatable. `--all` updates every root input and is
  exclusive with `--input`. One of them is required unless `--lock-file` is
  given.
- The generated call is exactly
  `nix flake update <input>... --flake <dir> --output-lock-file <temp>`.
  No other lock-writing command is called.
- `--lock-file <path>` uses that file as the after lock and calls no
  `nix flake update`. Combining it with `--input` or `--all` is an error.
- `--keep-lock <path>` copies the generated temp lock there. Combining it
  with `--lock-file` is an error.
- The before lock is read from `<flake>/flake.lock`. The tool reports the
  selected root inputs' `rev`, `lastModified`, and `narHash` before and after.
  `--all` reports every root input whose resolved locked object changed.
- A fixture proves the before `flake.lock` keeps its bytes and mtime after a
  successful run and after a failed run.
- The temporary lock is removed on success and on failure. `--keep-lock`
  copies it before cleanup.
- A failing `nix flake update` includes the command and Nix's stderr and
  exits 2. Fake-Nix tests assert the exact argv.


== Non-goals

- Evaluating derivations, parsing the delta, or parsing cost.
- Remote flake URLs; the flake directory must be local.
- Rewriting or committing the lock.


== Files

- `src/lock.rs`
- `src/nix.rs`
- `src/cli.rs`
- `src/main.rs`
- `tests/it/lock.rs`
- `tests/fixtures/locks/`

