// How this repo builds nix-bump-preview. Kept apart from goals.typ, which says
// what the tool is for. Every open task rates each goal here in `x-goals`,
// checked by `check-goals(...)` from truth/lib.typ.

#let goals = (
  test-first: (
    title: "Test before the behavior",
    acid-test: [Every behavior lands with a failing test first, or the task
      states why a test is impossible.],
  ),
  parsers-tested: (
    title: "Test every parser",
    acid-test: [Every parser has unit tests for the real format, including
      Unicode arrows, empty versions, multiple versions, and truncated lines. A
      parser never panics on input.],
  ),
  nix-only: (
    title: "Use only Nix",
    acid-test: [No runtime dependency on `nvd`, `dix`, `jq`, or a shell pipeline.
      The tool builds argv arrays and reads Nix's output.],
  ),
  no-personal: (
    title: "Keep private machine detail out",
    acid-test: [No local hostname, flake path, OS user name, system name, or
      machine-specific package set appears in code, docs, tests, fixtures, or
      tasks. Public repository metadata and placeholders are allowed.],
  ),
  focused: (
    title: "Orchestrate, do not reimplement",
    acid-test: [The tool does not walk the store, query binary caches, implement
      a cache checker, or reimplement a diff engine.],
  ),
)
