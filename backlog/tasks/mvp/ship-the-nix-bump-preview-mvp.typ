#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Ship the nix-bump-preview MVP",
  tags: ("nbp",),
  priority: 9,
  difficulty: 3,
  kind: "milestone",
  assignee: "agent",
  estimate: (src-files: 0),
  status: proposed(2026, 11, 4),
  links: depends-on(
    "implement-the-cli-contract-and-json-report-schema.typ",
    "resolve-a-proposed-lock-without-writing-flake-lock.typ",
    "evaluate-old-and-proposed-derivations-for-arbitrary-installables.typ",
    "parse-the-build-graph-package-delta.typ",
    "parse-the-build-and-fetch-cost.typ",
    "render-text-and-json-reports.typ",
    "harden-nix-invocation-errors-and-cleanup.typ",
    "add-end-to-end-tests-with-a-fake-nix.typ",
    "document-the-mvp-and-prepare-the-release.typ",
  ),
)

== Summary

The project is scaffolded but the CLI is still the template greeting. The MVP
must preview a flake input bump end to end: resolve a proposed lock without
touching `flake.lock`, evaluate old and proposed derivations, report package
version changes, report build and fetch cost, and emit stable JSON.

== Goals

- Product: `proposed-lock`, `package-delta`, `build-cost`, `no-build`,
  `generic`, `machine-readable`, `honest`.
- Process: `test-first`, `parsers-tested`, `nix-only`, `no-personal`,
  `focused`.

== Acceptance criteria

- Every child task is done and its acceptance criteria pass.
- `just check` is green on the merged result.
- `cargo run -- --help` shows the contract in `docs/design.md`.
- `--format json` validates against `schema/report.schema.json`.
- A repository search finds no private hostname, local flake path, OS user
  name, system name, or machine-specific package set in code, docs, tests,
  fixtures, or tasks. The public repository owner, repository URL, license
  attribution, dependency URLs, and placeholder examples are allowed.

== Non-goals

- Applying the update, switching, activating, or realizing outputs.
- Remote flake URLs.
- Exact runtime closures from `.drv` files alone.
- Replacing `canibump` or `nix-why-build`.

== Files

- `backlog/tasks/mvp/` (this milestone and its children)
- `docs/design.md`
- `docs/report.md`
- `docs/goals.md`
