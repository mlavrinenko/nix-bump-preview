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
from `docs/design.md` and the Rust report types that the rest of the MVP
fills in. This task owns argument parsing, help text, validation, and the
committed JSON schema. It does not run Nix yet.

Source of truth: `docs/design.md` sections Command, Options, JSON, Text, and
Exit codes.


== Goals

- Product: `generic`, `machine-readable`, `honest`.
- Process: `test-first`, `no-personal`, `focused`.


== Acceptance criteria

- `src/cli.rs` defines every option in `docs/design.md` with `clap`, including
  the hidden `--nix-bin` and the `--format text|json` choice.
- Missing `--installable`, missing `--input`/`--all` without `--lock-file`,
  and conflicting `--lock-file`/`--input` combinations exit 2 with a useful
  message.
- `src/report.rs` defines serde types for schema version, inputs,
  installables, delta changes, cost, attrs, and warnings.
- `schema/report.schema.json` is generated from those types with `schemars`
  and committed. A test fails when the committed schema and the generated
  schema differ.
- `cargo run -- --help` shows the contract. `--version` prints the crate
  version.
- Unit tests cover CLI validation and schema generation. `tests/it/cli.rs`
  covers help, version, and the invalid combinations above.
- No hostname, flake path, user name, or package set appears in the code,
  docs, tests, or fixtures.


== Non-goals

- Reading `flake.lock`, resolving a proposed lock, or running any Nix
  command.
- Parsing the delta or cost output.
- The final text layout; this task only defines the report types and schema.


== Files

- `src/cli.rs`
- `src/report.rs`
- `src/lib.rs`
- `src/main.rs`
- `schema/report.schema.json`
- `tests/it/cli.rs`
- `Cargo.toml`

