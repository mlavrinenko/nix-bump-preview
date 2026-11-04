# Contributing to nix-bump-preview

## Code Style

All clippy lints are set to `deny` level — the project will not compile with violations.

Key restrictions:
- No `unwrap()` — use `?` operator or `anyhow`/`thiserror` error handling
- No `todo!()`, `unimplemented!()`, `unreachable!()` — handle all cases
- No `unsafe` code
- No wildcard imports (`use foo::*`)
- No single-character variable names (minimum 2 characters)
- Functions: max 70 lines, max 5 arguments, max cognitive complexity 20

## Error Handling

- Use `anyhow::Result` for application-level code (binaries, CLI)
- Use `thiserror::Error` for library error types that callers will match on
- Propagate errors with `?` — never `unwrap()` or `expect()`

## Project Structure

Keep `main.rs` as a thin entry point — argument parsing, logger init, and a call into
library code. All logic belongs in `lib.rs` (and its modules). `main.rs` is excluded from
coverage, so anything there is untested by default.

## Goals, Tasks, and Recipes

The goals live in [truth/goals.typ](truth/goals.typ) and
[truth/process-goals.typ](truth/process-goals.typ); every open task rates each
in `x-goals`, managed with `mt`. Every public recipe carries `[doc("...")]` and
a group, the Justfile holds no comments, and longer bodies live in `.just/scripts/`.

## Testing

- Unit tests live inline in a `#[cfg(test)] mod tests` block next to the code they exercise.
- CLI and integration tests live in `tests/it/` and drive the built binary with
  [`assert_cmd`](https://docs.rs/assert_cmd) + [`predicates`](https://docs.rs/predicates)
  (see `tests/it/cli.rs`).
- `tests/it/` is the one integration binary: cargo builds an executable per
  file directly in `tests/`, and each one links every dependency again, so ten
  files are ten executables. A new test file goes in as `tests/it/<name>.rs`
  plus `mod <name>;` in `tests/it/main.rs`. Shared helpers go in `tests/it/common/mod.rs`, declared
  once in `main.rs` as `mod common;` and reached with `use crate::common::...`.
  `include_str!` and `#[path]` resolve relative to the source file;
  `env!("CARGO_MANIFEST_DIR")` paths do not move.
- Run the full suite with `just test`: `cargo nextest run` (one process per
  test, so a panic or hang is isolated; a green run prints only the summary, a
  failure prints the full output) plus `cargo test --doc` for doctests, which
  [cargo-nextest](https://nexte.st) does not run. `just cover` instruments the
  same nextest runner, so the gate and coverage can't disagree about whether a
  test passes. Both read the output level from
  [.config/nextest.toml](.config/nextest.toml), so neither recipe repeats it.
- As a file approaches the linecop limit, `just fix-check` ejects its inline
  `#[cfg(test)]` module into a sibling `_tests.rs` file via
  [ejectest](https://github.com/mlavrinenko/ejectest), driven by `linecop --baseline`.
  This keeps source files under the limit without losing the inline-test workflow.

## Code Coverage

Minimum 70% line coverage, enforced by `just cover` via `cargo-llvm-cov`. It
merges child-process profraw, so subprocess e2e tests attribute correctly. The
`cargo llvm-cov clean --workspace` is deliberate: `--no-report` keeps the pile,
and without the clean a long-lived checkout scores deleted binaries. `main.rs`
is excluded; keep it thin and move testable logic to `lib.rs`.

## CRAP Gate

`just crap` scores each function by the Change Risk Anti-Patterns metric
(cyclomatic complexity weighted by test coverage) and fails above 30. A global
coverage threshold can stay green while one branchy, untested function rots;
CRAP catches that. Fix a flagged function by adding tests or reducing its
branching. Tune the threshold per repo via `--threshold` or a `.cargo-crap.toml`.

It reads `target/coverage/lcov.info` and refuses a stale one, because cargo-crap
scores a function with no record as 0% covered. Run `just cover` first;
`.just/scripts/lcov-stale-reason.sh` is the check. The same rule excludes
`tests/**` and ejected `*_tests.rs` files, which llvm-cov never instruments.
`--format github` prints one line per failure and nothing on a green run.

## File Size Limits

Rust files: 500 lines max. Markdown files: 200 lines max. Split a file that
exceeds its limit into modules or separate documents.

## Dependency Drift

[outdatty.yaml](outdatty.yaml) declares groups that couple `source` files to the
`dependents` that must stay in sync with them — for example, CLI code to the docs
that describe it. `just check` runs `outdatty check`, which fails when a source
changed but its dependents were not re-confirmed.

After editing a source, review the listed dependents, update them as needed, run
`just outdatty-update` and commit `outdatty.lock`. Add a group whenever you
introduce files that must move together.

## Memoized Gates

Every arm of `just check` runs as `just mmz <subgate>`, which is
`mmz just <subgate>`. [mmz](https://github.com/mlavrinenko/mmz) hashes the rule's
declared inputs and skips the command when they are byte-for-byte identical to
the last run that *succeeded*, so a no-op `just check` re-run costs nothing and a
one-file change re-runs only the arms that declare that file.

Rules and their input scopes live in [.mmz/config.yaml](.mmz/config.yaml). Things
worth knowing:

- Adding an arm to `just check` means adding its rule to that manifest. mmz's
  default `strict` setting rejects an invocation matching no rule (and one whose
  rule resolves to zero files), so a missing rule is an error, not a silent pass.
- Rule names match by prefix on whole argv tokens. `just test` is `match: exact`
  so a filtered `just test <filter>` can never record the full-suite identity.
- Rules tagged `gate` are the gate set: `mmz --is-fresh --tag gate` asserts every
  one already passed against this worktree and runs nothing. That is what the
  pre-commit hook checks (see Git Hooks below), and what to wire into a task
  tracker's close gate. `just cover` and `just crap` are untagged — memoized,
  but never blocking. A tag matching no rule asserts nothing and passes
  vacuously, so any new gate arm must carry the `gate` tag to be covered.
- There is no `--force`. To re-run a fresh rule, touch one of its inputs or
  delete its record under `.mmz/cache/` (gitignored via `.mmz/.gitignore`).
- `just status` (`mmz --status`) prints every rule's freshness as a table — what
  a `just check` would re-run, and whether the pre-commit hook would pass now.

## Copy-Paste Gate

`just check-dry` runs [jscpd](https://github.com/kucherenko/jscpd) over `src/`
and `tests/` (`crates/` in a workspace) and fails on any duplicated Rust block of >=70 tokens. Fix a
finding by extracting a shared helper — never by shuffling tokens until the
detector loses the scent.

## Dependency Audits

`just deny` runs `cargo-deny` over advisories, the license allow-list, and
dependency bans. It is deliberately not part of `just check`: it fetches the
advisory database, so it needs network and would make an offline commit fail.
CI runs it in its own job. The recipe passes `--hide-inclusion-graph`: the
default prints an inclusion tree per crate it mentions, 1411 lines to say
`advisories ok, bans ok, licenses ok`. Drop the flag to chase one finding.

The policy lives in [deny.toml](deny.toml). A dependency under a license not in
`licenses.allow` fails the check — add one deliberately, not to silence a
finding. Keep the file in cargo-deny's current schema: a retired key (the old
`advisories.vulnerability`, `licenses.copyleft`, ...) is rejected outright and
fails the whole check rather than being ignored.

`just outdated` (cargo-outdated) lists dependencies with newer versions
published. Also network-bound, and never a gate: an upstream release is not a
reason for CI to go red. It is unrelated to `just outdatty-check` (doc drift).

## Releasing

`just release X.Y.Z --dry-run` first, then without the flag. Keep the entries
under `[Unreleased]`. [cargo-release](https://github.com/crate-ci/cargo-release)
(`release.toml`) bumps `Cargo.toml`, dates them, runs
`.just/scripts/release-hook.sh`, which holds every refusal, and commits. The
recipe pushes `main`, waits for `ci.yml` on it (`.just/scripts/release-ci.sh`)
and tags only if it is green. After a red run, fix `main` and re-run: the bump
stays done. The tag pushes through crates.io Trusted Publishing, so configure a
Trusted Publisher (owner, repo, workflow `release.yml`, environment `crates-io`)
before the first tag; no registry token secret exists. Never `cargo publish` by
hand.

## MSRV

The `rust-version` field in `Cargo.toml` declares the minimum supported Rust
version. CI verifies it compiles on that version in a separate job.

## Mutation Testing

`cargo-mutants` injects bugs and checks that the tests catch them. Two recipes:

- `just mutants-diff` — sweeps only mutants touched by this branch's diff
  against `main` (the daily driver).
- `just mutants` — full-file sweep (a dev aid, never a gate).

A surviving `MISSED` mutant is a real assertion gap: a line covered but not
pinned by any assertion.

## Git Hooks

The hook lives at `.githooks/pre-commit` and is tracked by git, so clones get
it without a copy step. `just install-hooks` runs
`git config core.hooksPath .githooks` (idempotent); the dev-shell `shellHook`
runs that recipe on entry, so a fresh clone's first `nix develop` wires the
hook and edits to the tracked file take effect live.

The hook runs `mmz --is-fresh --tag gate` — a freshness assertion that checks
every gate-tagged rule in `.mmz/config.yaml` already passed against this
worktree, running nothing of its own. It is cheap enough for every commit.
Bypass a single commit with `git commit --no-verify`.

## Submitting Changes

1. Run `just check` before submitting — it runs clippy, tests, file size, and drift checks
2. Run `just fmt` to format code
3. Ensure `just cover` meets the 70% threshold
