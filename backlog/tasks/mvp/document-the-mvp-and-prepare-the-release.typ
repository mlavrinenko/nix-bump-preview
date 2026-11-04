#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Document the MVP and prepare the release",
  tags: ("nbp", "docs", "tooling"),
  priority: 6,
  difficulty: 2,
  kind: "docs",
  assignee: "agent",
  estimate: (src-files: 1),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("add-end-to-end-tests-with-a-fake-nix.typ"),
)

== Summary

Make the repository explain itself and prove it is ready to release. The
README must match the shipped CLI and JSON, the changelog must name the MVP,
and every gate must pass on the merged result. No private machine detail may
survive anywhere.

Source of truth: `docs/goals.md`, `docs/design.md`, and `docs/report.md`.

== Goals

- Product: `machine-readable`, `honest`, `generic`.
- Process: `no-personal`, `test-first`.

== Acceptance criteria

- README usage, text output, JSON `jq` example, limits, and the `just check`
  description match the shipped behaviour. Examples use placeholders such as
  `<host>` and `nixosConfigurations.example`.
- `CHANGELOG.md` has an `[Unreleased]` entry naming the MVP.
- `docs/goals.md`, `docs/design.md`, `docs/report.md`, `AGENTS.md`, and
  `outdatty.yaml` agree with the code. `just outdatty-update` refreshes
  `outdatty.lock`; `www/index.html` and `CONTRIBUTING.md` are reviewed for
  drift.
- `just check`, `just cover`, and `just crap` are green. `just check` includes
  `check-tasks` when MindTape is on PATH and skips it with a notice otherwise.
- Release dry run: attempt `just release 0.1.0 --dry-run` only when a public
  `origin` exists, `gh auth status` succeeds, the tree is clean after this
  task's commit, and network is available. If `origin` is absent, run
  `cargo publish --dry-run --locked --allow-dirty` plus `just check`, record
  the missing remote as a blocker, and do not add a remote, publish, push, or
  tag.
- A repository search finds no private hostname, local flake path, OS user
  name, system name, or machine-specific package set in code, docs, tests,
  fixtures, or tasks. The public repository owner, repository URL, license
  attribution, dependency URLs, and placeholder examples are allowed.

== Non-goals

- Publishing to crates.io, pushing a tag, or creating a release.
- Wiring the tool into any user's NixOS configuration.
- New features after the MVP.

== Files

- `README.md`
- `CHANGELOG.md`
- `CONTRIBUTING.md`
- `AGENTS.md`
- `www/index.html`
- `outdatty.yaml`
- `outdatty.lock`
- `docs/goals.md`
- `docs/design.md`
- `docs/report.md`
