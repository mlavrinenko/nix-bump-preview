# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
