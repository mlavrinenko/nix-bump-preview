#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Resolve a proposed lock without writing flake.lock",
  tags: ("nbp", "lock", "cli"),
  priority: 8,
  difficulty: 4,
  kind: "feat",
  assignee: "agent",
  estimate: (src-files: 3),
  status: proposed(2026, 11, 4),
  links: parent("ship-the-nix-bump-preview-mvp.typ")
    + depends-on("implement-the-cli-contract-and-json-report-schema.typ"),
)

== Summary

Implement the lock module and the shared Nix runner. The runner is needed by
every later task, so it lands here: typed argv constructors for the safe calls,
`--nix-bin`/`NIX_BIN` resolution, and a fake-Nix fixture. The lock module reads
the before lock, resolves a proposed lock with `nix flake update
--output-lock-file`, or accepts an existing proposed lock with `--lock-file`.
The repository's `flake.lock` must stay byte-identical.

Source of truth: `docs/design.md` sections Options, Input reporting, Sequence,
Nix calls, and Safety invariants.

== Goals

- Product: `proposed-lock`, `no-build`, `generic`.
- Process: `test-first`, `nix-only`, `no-personal`.

== Acceptance criteria

- `src/nix.rs` exposes typed constructors for the calls in `docs/design.md`.
  No caller builds a raw command. `--nix-bin` wins over `NIX_BIN`, both win
  over `nix`, and a relative path is resolved against the current directory.
  A missing binary is a clear error, not a panic.
- `tests/fixtures/fake-nix` is a portable script that records argv and prints
  canned output. Tasks 3-7 use it.
- `--input <name>` is repeatable. `--all` updates every root input and is
  exclusive with `--input`. One of them is required unless `--lock-file` is
  given.
- The generated call is exactly
  `nix flake update <input>... --flake <dir> --output-lock-file <temp>`.
  `--all` calls `nix flake update --flake <dir> --output-lock-file <temp>` with
  no input names. No other lock-writing command is called.
- `--lock-file <path>` uses that file as the after lock and calls no
  `nix flake update`. Combining it with `--input` or `--all` is an error.
- `--keep-lock <path>` copies the generated temp lock there, creates missing
  parent directories, and overwrites an existing file. Combining it with
  `--lock-file` is an error.
- Validate `--flake` before any Nix call: it must be a local directory with a
  `flake.lock`. Canonicalize it to an absolute path for the report. Remote
  URLs, missing directories, and missing locks exit 2.
- Read the before lock from `<flake>/flake.lock`. Report every selected input
  with a `changed` boolean. `changed` means the resolved `locked` object
  differs in any field. Missing `rev`, `lastModified`, or `narHash` is omitted,
  not invented; a path input without `rev` must serialize. Sort inputs by name.
  With `--lock-file`, every root input is selected; with `--input`, only the
  named inputs; with `--all`, every root.
- The temporary lock is removed on success and on failure. A fixture proves
  the before `flake.lock` keeps its bytes and mtime after a successful run and
  after a failed run.
- A failing `nix flake update` includes the command and Nix's stderr and exits
  2. Fake-Nix tests assert the exact argv.
- `src/lib.rs` declares `pub mod nix;` and `pub mod lock;`.
  `tests/it/main.rs` declares `mod lock;`.
- Run `just outdatty-update` and commit the refreshed `outdatty.lock`.

== Non-goals

- Evaluating derivations, parsing the delta, or parsing cost.
- Remote flake URLs; the flake directory must be local.
- Rewriting or committing the lock.

== Files

- `src/lock.rs`
- `src/nix.rs`
- `src/lib.rs`
- `src/cli.rs`
- `src/main.rs`
- `tests/it/lock.rs`
- `tests/it/main.rs`
- `tests/fixtures/fake-nix`
- `tests/fixtures/locks/` (including a path input with no `rev`)
- `outdatty.lock`
