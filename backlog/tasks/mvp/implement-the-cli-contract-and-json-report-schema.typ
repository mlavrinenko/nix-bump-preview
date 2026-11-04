#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Implement the CLI contract and JSON report schema",
  tags: ("nbp", "cli", "json"),
  priority: 9,
  difficulty: 4,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 4),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ"),
)

== Summary

The binary is still the template greeting. Implement the full CLI surface
from `docs/design.md` and the report types from `docs/report.md`. This task
owns argument parsing, help text, validation, schema generation, and the
module skeleton. It does not run Nix yet.

Source of truth: `docs/design.md` sections Command, Options, and JSON schema;
`docs/report.md` section JSON.

== Goals

- Product: `generic`, `machine-readable`, `honest`.
- Process: `test-first`, `no-personal`, `focused`.

== Acceptance criteria

- `src/cli.rs` defines every option in `docs/design.md` with `clap`, including
  the hidden `--print-schema`, the hidden `--nix-bin`, and the
  `--format text|json` choice. `--nix-bin` reads `NIX_BIN` through clap's
  `env` feature.
- Missing `--installable`, missing `--input`/`--all` without `--lock-file`,
  conflicting `--lock-file`/`--input`, and `--keep-lock` with `--lock-file`
  exit 2 with a useful message.
- `src/report.rs` freezes the complete JSON object in `docs/report.md`: every
  field, camelCase name, optional field, array order, and omission rule.
  `schemaVersion` is 1. `rev`, `lastModified`, and `narHash` are optional
  strings or integers, so a path input with no `rev` still serializes.
- `schema/report.schema.json` is generated with `schemars` and committed.
  `--print-schema` works with no other argument, is handled before validation,
  prints the generated schema, and exits 0. A test compares the printed schema
  to the committed file and fails on drift. Add a `just schema` recipe that
  regenerates the file through the binary.
- `src/lib.rs` declares `pub mod cli;` and `pub mod report;`. `tests/it/main.rs`
  declares `mod cli;`. Keep `main.rs` thin: parse, initialise logging, call
  `run`, map the error to exit 2.
- A valid invocation before the pipeline exists exits 2 with
  `error: preview pipeline is not implemented yet`; it never exits 0 silently.
- Remove the template `greet` function, `AppError::Io` if it becomes unused,
  the inline `test_greet`, and `greets_provided_name`. Keep the `--version`
  test.
- Add `serde`, `serde_json`, and `schemars` as dependencies, and enable clap's
  `env` feature.
- Run `just outdatty-update` and commit the refreshed `outdatty.lock`. Add the
  `[Unreleased]` changelog entry.
- No private machine detail appears. The public repository owner, repository
  URL, license attribution, dependency URLs, and placeholders such as
  `<host>` and `nixosConfigurations.example` are allowed.

== Non-goals

- Reading `flake.lock`, resolving a proposed lock, or running any Nix command.
- Parsing the delta or cost output.
- The final text layout; task 6 owns the rendered output.

== Files

- `src/cli.rs`
- `src/report.rs`
- `src/lib.rs`
- `src/main.rs`
- `schema/report.schema.json`
- `tests/it/cli.rs`
- `tests/it/main.rs`
- `Cargo.toml`
- `Cargo.lock`
- `Justfile`
- `CHANGELOG.md`
- `outdatty.lock`
