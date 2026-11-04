#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Render text and JSON reports",
  tags: ("nbp", "json", "cli", "delta", "cost"),
  priority: 8,
  difficulty: 3,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 3),
  status: done(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("harden-nix-invocation-errors-and-cleanup.typ"),
)

== Summary

Own the end-to-end run and both output formats. `lib::run(cli)` executes
`docs/design.md` Sequence steps 1-7, and `main.rs` only parses, initialises
logging, calls `run`, and maps the result to an exit code. Text is for people,
JSON is the scripting surface, and both carry the same facts. Add
`--exit-code`.

Source of truth: `docs/design.md` sections Sequence and JSON schema;
`docs/report.md` sections JSON, Text, and Exit codes.

== Goals

- Product: `machine-readable`, `honest`, `generic`.
- Process: `test-first`, `no-personal`.

== Acceptance criteria

- `lib::run(cli)` executes the full sequence: validate and read the before
  lock, resolve or accept the after lock, compare inputs, evaluate
  installables and attrs, run the delta and cost, render, clean up.
- `--no-delta` skips the `nix store diff-closures` call, not only its output.
  `--no-cost` skips the `nix build --dry-run` call, not only its output. The
  fake-Nix argv log proves it.
- A failure before rendering emits no partial report and exits 2.
- `--format json` prints one pretty-printed object with struct-order keys and
  camelCase names, exactly as `docs/report.md` shows. `delta`, `cost`, `attrs`,
  and `warnings` are omitted when empty or not computed.
- `--print-schema` and the committed `schema/report.schema.json` match the
  final types. Update both together or not at all; the drift test must fail if
  they diverge.
- Text output follows `docs/report.md`: input lines for changed and unchanged
  inputs, each installable in CLI order, every parsed change, attr lines,
  cost, and each warning as `warning: ...`. Revisions are eight characters,
  dates are UTC `YYYY-MM-DD`, and added/removed/multi-version changes use the
  exact forms in the document.
- `--exit-code` follows `docs/report.md` and only considers what was computed.
- Tests cover text snapshots, JSON snapshots, schema validation, and each
  exit-code case.
- `src/lib.rs` owns `run`; `main.rs` stays thin. `tests/it/main.rs` declares
  `mod report;`.
- Run `just outdatty-update` and commit the refreshed `outdatty.lock`.
- Add a `jq` example to the README that runs against a fixture report.

== Non-goals

- New Nix calls.
- Terminal colour, progress bars, or interactive output.
- Changing the report contract without updating `docs/report.md` and the
  schema in the same commit.

== Files

- `src/report.rs`
- `src/lib.rs`
- `src/main.rs`
- `docs/report.md`
- `schema/report.schema.json`
- `tests/it/report.rs`
- `tests/it/main.rs`
- `README.md`
- `outdatty.lock`
