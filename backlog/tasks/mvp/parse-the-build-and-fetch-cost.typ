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
    + depends-on("resolve-a-proposed-lock-without-writing-flake-lock.typ"),
)

== Summary

Implement the cost module. It runs `nix build --dry-run` against each
installable with the proposed lock and parses the two plan headers into
counts and byte sizes. It must never call `nix build` without `--dry-run`.

Source of truth: `docs/design.md` section Cost parsing.


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
  A missing header is zero. Singular and plural headers both work.
- Cost is reported per installable and is not merged across installables.
- A fake-Nix test asserts `--dry-run` and `--no-write-lock-file` are present,
  and that no command without `--dry-run` can be produced.
- Unit tests cover zero, one, and many paths, all size units, and malformed
  sizes.


== Non-goals

- The package delta.
- Rendering text or JSON.
- Walking the store or querying substituters directly; Nix answers the plan.


== Files

- `src/cost.rs`
- `src/report.rs`
- `tests/it/cost.rs`

