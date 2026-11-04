// What nix-bump-preview is for. Every open task rates each goal in its
// `x-goals` field, checked by `check-goals(...)` from truth/lib.typ. How this
// repo builds the tool is a separate set, in process-goals.typ.

#let goals = (
  proposed-lock: (
    title: "Preview a proposed lock",
    acid-test: [After a run, the lock file's bytes and mtime are unchanged, and
      `git status` shows no change caused by the tool. `--lock-file` takes an
      already-proposed lock instead of resolving one.],
  ),
  package-delta: (
    title: "Show the package delta",
    acid-test: [A fixture with a realized new output still reports the version
      change. The delta comes from the build-time derivation graph, not from a
      store-presence check.],
  ),
  build-cost: (
    title: "Show the build cost",
    acid-test: [The report distinguishes built from fetched derivations, parses
      `B`/`KiB`/`MiB`/`GiB`, and reports zero when there is nothing to do.],
  ),
  no-build: (
    title: "Never build the target",
    acid-test: [A fake Nix records every call: no `nix build` without
      `--dry-run`, no `nix store diff-closures` without `--derivation`, and no
      activation command at all.],
  ),
  generic: (
    title: "Stay generic",
    acid-test: [Integration tests use two unrelated flakes and installables, and
      a repository search finds no private hostname, local flake path, OS user
      name, or machine-specific package set.],
  ),
  machine-readable: (
    title: "Carry the same facts in JSON",
    acid-test: [`--format json` validates against `schema/report.schema.json`,
      and the README shows a `jq` query over the real shape.],
  ),
  honest: (
    title: "Say what the preview cannot know",
    acid-test: [Text and JSON carry a warning naming the build-time
      approximation, and the README states the limit.],
  ),
)

#let non-goals = (
  "Applying, switching, activating, or realizing an update.",
  "Auditing vulnerabilities or licences.",
  "Replacing `canibump` (upstream readiness) or `nix-why-build` (why a build
    misses the cache).",
  "Producing an exact runtime closure from `.drv` files alone.",
)
