# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-04

### Changed

- The README is consumer-facing: the status section is gone, the delta limit
  lives in How it works, and Install documents the flake input and `nix run`.
- The product and process goals are tracked Typst modules under `truth/`, and
  every open task rates them in `x-goals` instead of a prose Goals section.
- The Justfile documents every recipe with `[doc("...")]`, holds no comments,
  and keeps its longer bodies in `.just/scripts/`.
- `Cargo.toml` carries crates.io keywords and a second category, so the crate
  is discoverable.
- The release workflow publishes through crates.io Trusted Publishing instead
  of an expiring `CARGO_REGISTRY_TOKEN` secret.

### Fixed

- A green `just crap` prints nothing: the gate uses cargo-crap's `github`
  format and keeps one line per failing function.
- `just release` works against an empty origin. The hook no longer refuses a
  repository whose first push creates `main`.
- The release workflow accepts a scoped `CARGO_REGISTRY_TOKEN` for the first
  publish, when crates.io cannot have a Trusted Publisher yet, then switches to
  OIDC once the secret is gone.

### Added

- The MVP: preview a Nix flake input bump without building or switching,
  reporting package version deltas and build and fetch cost in text or JSON.
- Project scaffold from cratemplate, product and process goals, the CLI and
  JSON design contract, and the MVP backlog.
- The full CLI surface with the documented usage validation, and the frozen
  JSON report types, committed schema, and `just schema` regeneration recipe.
- The lock resolver, the typed Nix runner with `--nix-bin`/`NIX_BIN`
  resolution, the fake-Nix test fixture, and fixture locks proving `flake.lock`
  stays untouched. Follows roots resolve to their target, and an empty root is
  a valid lock.
- Installable and attr evaluation against the before and proposed locks, with
  the exact `nix path-info` and `nix eval` argv and no build or activation
  call.
- The build-graph package delta parser for
  `nix store diff-closures --derivation`, grouping versions by package,
  accepting both arrow glyphs and the empty-version markers, stripping size
  suffixes, and warning about malformed lines.
- The build and fetch cost parser for `nix build --dry-run`, reading both plan
  headers, summing repeated headers, accepting singular and plural counts, and
  converting binary-unit sizes with exact half-up rounding.
- The end-to-end preview pipeline and both report formats, with the same facts
  in text and JSON. `--no-delta` and `--no-cost` skip their Nix calls, the delta
  approximation is stated as a warning, and `--exit-code` reports changes and
  build cost.
