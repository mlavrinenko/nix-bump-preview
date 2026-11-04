#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Parse the build and fetch cost",
  tags: ("nbp", "cost"),
  priority: 7,
  difficulty: 3,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 3),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("parse-the-build-graph-package-delta.typ"),
)

== Summary

Implement the cost module. It runs `nix build --dry-run` against each
installable with the proposed lock and parses the two plan headers into
counts and byte sizes. It must never call `nix build` without `--dry-run`.
This task depends on the delta task so the two parsers land in one order and
do not race on the same files. It does not edit `src/report.rs`.

Source of truth: `docs/report.md` section Cost parsing.

== Goals

- Product: `build-cost`, `no-build`, `machine-readable`.
- Process: `test-first`, `parsers-tested`, `nix-only`.

== Acceptance criteria

- The command is
  `nix build --dry-run --no-write-lock-file --reference-lock-file <after>
  <flake>#<installable>`, with stdout discarded and stderr captured.
- The parser reads `these N derivations will be built:` and
  `these M paths will be fetched (X download, Y unpacked):`.
- `B`, `KiB`, `MiB`, `GiB`, and `TiB` are accepted with binary multipliers.
  A fractional size is parsed as a decimal and rounded half up to whole bytes.
  An overflow beyond `u64` is an error and exits 2. A missing header is zero.
  Singular and plural headers both work.
- Cost is reported per installable and is not merged across installables.
- A fake-Nix test asserts `--dry-run` and `--no-write-lock-file` are present,
  and that no command without `--dry-run` can be produced.
- Commit captured real `nix build --dry-run` stderr at
  `tests/fixtures/captured/dry-run.txt` and use it in unit tests, alongside
  zero, one, and many paths, every size unit, fractional sizes, overflow, and
  malformed sizes.
- `src/lib.rs` declares `pub mod cost;`. `tests/it/main.rs` declares
  `mod cost;`.
- Run `just outdatty-update` and commit the refreshed `outdatty.lock`.

== Non-goals

- The package delta.
- Rendering text or JSON.
- Walking the store or querying substituters directly; Nix answers the plan.
- Editing `src/report.rs` or the committed schema.

== Files

- `src/cost.rs`
- `src/lib.rs`
- `tests/it/cost.rs`
- `tests/it/main.rs`
- `tests/fixtures/captured/dry-run.txt`
- `outdatty.lock`
