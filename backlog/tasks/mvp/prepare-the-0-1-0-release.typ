#import "@local/mindtape:0.2.0": *
#import "@local/nix-bump-preview:0.1.0": helps, hurts, neutral, verdicts

#show: task.with(
  title: "Prepare the 0.1.0 release",
  tags: ("release", "docs", "tooling"),
  priority: 5,
  x-goals: (
    product: verdicts(
      (
        generic: helps[the install docs use the public repository and placeholders],
        honest: helps[the delta limit is stated in How it works],
      ),
      ..(
        "proposed-lock",
        "package-delta",
        "build-cost",
        "no-build",
        "generic",
        "machine-readable",
        "honest",
      ),
    ),
    process: verdicts(
      (
        no-personal: helps[the install example carries no machine path],
      ),
      ..(
        "test-first",
        "parsers-tested",
        "nix-only",
        "no-personal",
        "focused",
      ),
    ),
  ),
  difficulty: 2,
  kind: "docs",
  assignee: "agent",
  estimate: (src-files: 0),
  status: done(2026, 11, 5),
)

== Summary

The MVP is code-complete, but the consumer surface is not ready for a 0.1.0
tag. The crate has no keywords, the README still carries an internal status
section and development recipes, the goals are Markdown prose no task check
reads, the Justfile is a comment-heavy script pile, and the release workflow
still depends on an expiring crates.io token.

== Acceptance criteria

- `Cargo.toml` carries discoverable keywords and valid categories, and
  `cargo publish --dry-run --locked` passes.
- The README keeps consumer information only: no Status section, the delta
  limit inside How it works, a short flake-input or `nix run` install, and
  Development naming `CONTRIBUTING.md`.
- `flake.nix` exposes `packages.<system>.default`, so a nixpkgs-style input can
  put it in `home.packages`, and `nix run github:mlavrinenko/nix-bump-preview`
  starts the binary.
- `truth/goals.typ`, `truth/process-goals.typ`, and `truth/lib.typ` are the
  tracked goals; every open task rates them in `x-goals`, and `mt check` fails
  a task that misses or helps no goal.
- The Justfile has `[doc(...)]` on every public recipe, no comments, and the
  large bodies live under `.just/scripts/`; every recipe still works.
- The release workflow uses crates.io Trusted Publishing, with no long-lived
  token secret.
- `just check`, `just cover`, `just crap`, and `just deny` are green, and
  `just release 0.1.0 --dry-run` passes.

== Non-goals

- Publishing, pushing a tag, or cutting the GitHub release. The user runs the
  release after this task lands.
- Adding new preview behavior or changing the CLI, JSON, or report schema.
- Adding an overlay, NixOS module, or Home Manager module.

== Files

- `Cargo.toml`
- `README.md`
- `www/index.html`
- `flake.nix`
- `Justfile`
- `.just/scripts/`
- `truth/`
- `.mindtape/config.toml`
- `.mindtape/templates/task.typ`
- `.github/workflows/release.yml`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `outdatty.yaml`
- `outdatty.lock`
- `.mmz/config.yaml`
- `CHANGELOG.md`
