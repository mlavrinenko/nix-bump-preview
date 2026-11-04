#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Parse the build-graph package delta",
  tags: ("nbp", "delta"),
  priority: 8,
  difficulty: 3,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 3),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on(
      "evaluate-old-and-proposed-derivations-for-arbitrary-installables.typ",
    ),
)

== Summary

Implement the delta module. It runs
`nix store diff-closures --derivation <before.drv> <after.drv>` and parses
package version changes into the report type frozen by task 1. The command
must always carry `--derivation`; without it Nix realises the new closure.
This task does not edit `src/report.rs`.

Source of truth: `docs/report.md` section Delta parsing.

== Goals

- Product: `package-delta`, `no-build`, `honest`.
- Process: `test-first`, `parsers-tested`, `nix-only`.

== Acceptance criteria

- The parser handles both `->` and `→`, `∅` and `ε`, comma-separated version
  lists, duplicate package names, and size suffixes after the versions.
- Group by package name, union the before and after version sets, and keep one
  entry when the sets differ. A size-only line is skipped without a warning.
- A malformed line appends exactly
  `warning: ignored malformed diff line: <line>` to the top-level warning
  list, in input order. The parser never panics.
- Changes are sorted by package name and classified as `added`, `removed`, or
  `changed`.
- The command builder always includes `--derivation`. A fake-Nix test asserts
  that a command without it cannot be produced.
- Commit captured real output at
  `tests/fixtures/captured/diff-closures.txt` and use it in unit tests,
  alongside the examples in `docs/report.md` and malformed/truncated lines.
- An already-realised new output still appears in the delta, because the
  parser reads the derivation graph and never checks the local store.
- `src/lib.rs` declares `pub mod delta;`. `tests/it/main.rs` declares
  `mod delta;`.
- Run `just outdatty-update` and commit the refreshed `outdatty.lock`.

== Non-goals

- The build/fetch cost.
- Rendering text or JSON.
- Reimplementing Nix's closure diff or querying binary caches.
- Editing `src/report.rs` or the committed schema.

== Files

- `src/delta.rs`
- `src/lib.rs`
- `tests/it/delta.rs`
- `tests/it/main.rs`
- `tests/fixtures/captured/diff-closures.txt`
- `outdatty.lock`
