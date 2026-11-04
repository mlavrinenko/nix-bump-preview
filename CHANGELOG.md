# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Project scaffold from cratemplate, product and process goals, the CLI and
  JSON design contract, and the MVP backlog.
- The full CLI surface with the documented usage validation, and the frozen
  JSON report types, committed schema, and `just schema` regeneration recipe.
- The lock resolver, the typed Nix runner with `--nix-bin`/`NIX_BIN`
  resolution, the fake-Nix test fixture, and fixture locks proving `flake.lock`
  stays untouched.
