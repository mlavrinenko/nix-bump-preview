#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Evaluate old and proposed derivations for arbitrary installables",
  tags: ("nbp", "eval", "cli"),
  priority: 8,
  difficulty: 4,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 3),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("resolve-a-proposed-lock-without-writing-flake-lock.typ"),
)

== Summary

Implement derivation evaluation. For every `--installable`, evaluate the
before `.drv` with the current lock and the after `.drv` with the proposed
lock. For every `--attr`, evaluate `.version` against both locks. No Nix
command in this task may build an output.

Source of truth: `docs/design.md` sections Options, Sequence, and Nix calls.


== Goals

- Product: `package-delta`, `no-build`, `generic`.
- Process: `test-first`, `nix-only`, `no-personal`.


== Acceptance criteria

- The before call is
  `nix path-info --derivation --no-write-lock-file <flake>#<installable>`.
  The after call adds `--reference-lock-file <after>`.
- `--installable` is repeatable and required. Every result carries the attr,
  the before `.drv` path, and the after `.drv` path.
- `--attr` is repeatable and optional. Each result carries the attr, the
  before version, and the after version, read with
  `nix eval --raw --no-write-lock-file [--reference-lock-file <after>]
  <flake>#<attr>.version`.
- A missing installable, missing attr, or attr without `.version` includes
  Nix's stderr and exits 2 without emitting a partial report.
- Fake-Nix tests assert the exact argv for both locks and prove that no
  command can build an output.
- The implementation never assumes NixOS, home-manager, a system name, or a
  package-set layout. Fixtures use two unrelated installables.


== Non-goals

- Parsing the package delta or the build cost.
- Realising the derivations or reading the local store.


== Files

- `src/eval.rs`
- `src/cli.rs`
- `src/report.rs`
- `tests/it/eval.rs`

