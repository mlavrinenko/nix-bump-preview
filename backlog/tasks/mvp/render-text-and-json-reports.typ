#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Render text and JSON reports",
  tags: ("nbp", "json", "cli", "delta", "cost"),
  priority: 8,
  difficulty: 3,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 3),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on(
      "parse-the-build-graph-package-delta.typ",
      "parse-the-build-and-fetch-cost.typ",
    ),
)

== Summary

Wire the report types to the two output formats. Text is for people, JSON is
the scripting surface, and both carry the same facts. Add `--exit-code`.
The JSON schema is already committed by the CLI task; this task keeps it in
sync with the final shape.

Source of truth: `docs/design.md` sections JSON, Text, and Exit codes.


== Goals

- Product: `machine-readable`, `honest`, `generic`.
- Process: `test-first`, `no-personal`.


== Acceptance criteria

- `--format json` prints one object with `schemaVersion`, `flake`, `inputs`,
  `installables`, and `attrs`. `delta`, `cost`, `attrs`, and `warnings` are
  omitted when empty or not computed.
- The JSON validates against `schema/report.schema.json`. The committed
  schema matches the Rust types; a drift test fails.
- Text output follows `docs/design.md`: input revisions, each installable,
  every parsed change, the cost line, and the approximation warning.
- `--no-delta` and `--no-cost` omit their sections and do not print their
  headings.
- `--exit-code` exits 1 when an input changed, a package changed, or
  `toBuild + toFetch > 0`; it exits 0 when nothing changed.
- A `jq` example in the README runs against a fixture report.
- Tests cover text snapshots, JSON snapshots, schema validation, and each
  exit-code case.


== Non-goals

- New Nix calls.
- Terminal colour, progress bars, or interactive output.


== Files

- `src/report.rs`
- `src/main.rs`
- `schema/report.schema.json`
- `tests/it/report.rs`
- `README.md`

