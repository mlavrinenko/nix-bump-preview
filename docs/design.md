# Design

This document is the source of truth for the command, the run sequence, the Nix
calls, and the safety invariants. The delta, cost, JSON, text, and test
contracts are in [report.md](report.md). A task that changes either contract
updates the document and its examples in the same commit.

## Command

```text
nix-bump-preview --input <name>... --installable <attr>... [options]
```

`--flake` defaults to `.` and must be a local directory containing
`flake.lock`. The path is canonicalized before it enters the report. Remote
flake URLs, missing directories, and directories without a lock exit 2 before
any Nix call.

## Options

| Option | Meaning |
| --- | --- |
| `-f, --flake <DIR>` | Local flake directory, default `.` |
| `-i, --input <NAME>` | Root input to update; repeatable |
| `--all` | Update every root input; exclusive with `--input` |
| `-l, --lock-file <PATH>` | Use an existing proposed lock; skip `nix flake update` |
| `--installable <ATTR>` | Attr under `$flake` to diff; repeatable, required |
| `--attr <ATTR>` | Attr with a `.version` for a direct version delta; repeatable |
| `--format <text\|json>` | Output format, default `text` |
| `--keep-lock <PATH>` | Copy the generated proposed lock to `PATH` |
| `--no-delta` | Skip the package delta and its Nix call |
| `--no-cost` | Skip the build and fetch cost and its Nix call |
| `--exit-code` | Exit 1 when a change or build cost exists, else 0 |
| `--nix-bin <PATH>` | Hidden. Nix executable, default `nix`; flag wins over `NIX_BIN` |
| `--print-schema` | Hidden. Print the JSON schema and exit 0; no other argument is required |
| `-h, --help`, `-V, --version` | Standard |

`--input` or `--all` is required unless `--lock-file` or `--print-schema` is given. `--print-schema` is handled before validation and exits 0. `--lock-file`
with `--input` or `--all` is an error. `--keep-lock` with `--lock-file` is an
error. `--keep-lock` creates missing parent directories and overwrites an
existing file. The temp lock lives in the system temp directory and is removed
after the run unless `--keep-lock` copied it.

### Input reporting

The selected inputs are the names given with `--input`, every root input with
`--all`, and every root input with `--lock-file`. The report includes every
selected input with a `changed` boolean. `changed` means the resolved `locked`
object differs in any field; missing `lastModified` or `narHash` is omitted,
not invented. Inputs are sorted by name. Unchanged inputs are still listed in
JSON and text, with `changed: false`.

## Sequence

1. Validate `--flake` and read `$flake/flake.lock` as the before lock.
2. If `--lock-file` is absent, run `nix flake update` for the named inputs or
   `--all`, writing only `--output-lock-file <temp>`. If present, use it as the
   after lock and resolve nothing.
3. Compare every selected input's resolved `locked` object in both locks.
4. For each `--installable`, evaluate the old `.drv` with the before lock and
   the new `.drv` with the after lock.
5. Unless `--no-delta`, run the derivation-graph delta and parse it. Unless
   `--no-cost`, run the dry-run plan and parse it.
6. For each `--attr`, evaluate `.version` against both locks and report it.
7. Render text or JSON. Remove the temporary lock unless `--keep-lock` copied
   it.

`lib::run(cli)` owns steps 1-7. `main.rs` parses arguments, initialises logging,
calls `run`, and maps the result to an exit code. `--no-delta` and `--no-cost`
must skip their Nix calls, not only their output sections.

Installables are reported in CLI order. Attrs are reported in CLI order.
Package changes are sorted by name. Inputs are sorted by name.

## Nix calls

Every call is an argv array. Never pass a shell string to `sh -c`.

```text
nix flake update <input>... --flake <flake> --output-lock-file <tmp>
nix flake update --flake <flake> --output-lock-file <tmp>          # --all
nix path-info --derivation --no-write-lock-file <flake>#<installable>
nix path-info --derivation --no-write-lock-file \
  --reference-lock-file <after> <flake>#<installable>
nix store diff-closures --derivation <old.drv> <new.drv>
nix build --dry-run --no-write-lock-file \
  --reference-lock-file <after> <flake>#<installable>
nix eval --raw --no-write-lock-file <flake>#<attr>.version
nix eval --raw --no-write-lock-file \
  --reference-lock-file <after> <flake>#<attr>.version
```

`nix flake update` and `nix build` may download flake sources and cache
metadata. They must not realize the target outputs. `nix store diff-closures`
must always carry `--derivation`; without it Nix realizes the new closure.

`--nix-bin` wins over `NIX_BIN`; both win over the default `nix`. A relative
path is resolved against the current working directory before spawning.

## Safety invariants

- `flake.lock` is never written. Only `--output-lock-file` and
  `--no-write-lock-file` are used.
- No `nix build` without `--dry-run`.
- No `nix store diff-closures` without `--derivation`.
- No `switch`, `boot`, `test`, `activate`, or profile command.
- The temporary lock is removed on success and on failure. `--keep-lock`
  copies it before cleanup.
- Nix stderr is captured and included in the error. A failed call names the
  argv and exits 2.
- A failure before rendering emits no partial report.

## JSON schema

`schema/report.schema.json` is generated from the Rust types and committed.
`--print-schema` prints the generated schema. A test compares the printed
schema to the committed file and fails on drift.

## Out of scope

- Applying the update or switching the system.
- Remote flake URLs in the MVP.
- Exact runtime closures without realized outputs.
- Cache walking, vulnerability checks, or a replacement for `nix-why-build`.
