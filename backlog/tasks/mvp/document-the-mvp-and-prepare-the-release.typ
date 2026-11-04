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
and every gate must pass on the merged result. No personal system reference
may survive anywhere.

Source of truth: `docs/goals.md` and `docs/design.md`.


== Goals

- Product: `machine-readable`, `honest`, `generic`.
- Process: `no-personal`, `test-first`.


== Acceptance criteria

- README usage, text output, JSON `jq` example, and limits match the shipped
  behaviour. Examples use placeholders such as `<host>`, never a real
  hostname or path.
- `CHANGELOG.md` has an `[Unreleased]` entry naming the MVP.
- `docs/goals.md`, `docs/design.md`, `AGENTS.md`, and `outdatty.yaml` agree
  with the code. `just outdatty-update` refreshes `outdatty.lock`.
- `just check`, `just cover`, and `just crap` are green.
- `just release 0.1.0 --dry-run` passes when network is available; do not
  publish or push from this task.
- A repository search for personal hostnames, flake paths, user names, and
  system names finds nothing in code, docs, tests, fixtures, or tasks.


== Non-goals

- Publishing to crates.io, pushing a tag, or creating a release.
- Wiring the tool into any user's NixOS configuration.
- New features after the MVP.


== Files

- `README.md`
- `CHANGELOG.md`
- `AGENTS.md`
- `outdatty.yaml`
- `outdatty.lock`
- `docs/goals.md`
- `docs/design.md`

