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
package version changes into the report type. The command must always carry
`--derivation`; without it Nix realises the new closure.

Source of truth: `docs/design.md` section Delta parsing.


== Goals

- Product: `package-delta`, `no-build`, `honest`.
- Process: `test-first`, `parsers-tested`, `nix-only`.


== Acceptance criteria

- The parser handles both `->` and `→`, `∅` and `ε`, comma-separated version
  lists, duplicate package names, and size suffixes after the versions.
- A line is kept only when the before and after version sets differ. A
  size-only line is skipped. A malformed line produces a warning and never a
  panic.
- Changes are sorted by package name and classified as `added`, `removed`, or
  `changed`.
- The command builder always includes `--derivation`. A fake-Nix test asserts
  that a command without it cannot be produced.
- Unit tests cover the real examples in `docs/design.md`, a captured
  `nix store diff-closures` run, and malformed/truncated lines.
- An already-realised new output still appears in the delta, because the
  parser reads the derivation graph and never checks the local store.


== Non-goals

- The build/fetch cost.
- Rendering text or JSON.
- Reimplementing Nix's closure diff or querying binary caches.


== Files

- `src/delta.rs`
- `src/report.rs`
- `tests/it/delta.rs`

