# Design

This document is the source of truth for the CLI and report contract. A task
that changes either updates this file and its examples in the same commit.

## Command

```text
nix-bump-preview --input <name>... --installable <attr>... [options]
```

`--flake` defaults to `.` and must be a local directory. Resolving a remote
flake is out of scope for the MVP.

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
| `--no-delta` | Skip the package delta |
| `--no-cost` | Skip the build and fetch cost |
| `--exit-code` | Exit 1 when a change or build cost exists, else 0 |
| `--nix-bin <PATH>` | Nix executable, default `nix`; also `NIX_BIN`; hidden |
| `-h, --help`, `-V, --version` | Standard |

`--input` or `--all` is required unless `--lock-file` is given. `--lock-file`
with `--input` or `--all` is an error. `--keep-lock` with `--lock-file` is an
error. `--flake` must contain `flake.lock`.

## Sequence

1. Read `$flake/flake.lock` as the before lock.
2. If `--lock-file` is absent, run `nix flake update` for the named inputs or
   `--all`, writing only `--output-lock-file <temp>`. If present, use it as the
   after lock and resolve nothing.
3. Compare every selected root input's resolved `locked` object in the before
   and after locks. Report the input names that changed.
4. For each `--installable`, evaluate the old `.drv` with the before lock and
   the new `.drv` with the after lock.
5. Unless `--no-delta`, run the derivation-graph delta and parse it. Unless
   `--no-cost`, run the dry-run plan and parse it.
6. For each `--attr`, evaluate `.version` against both locks and report it.
7. Render text or JSON. Remove the temporary lock unless `--keep-lock` copied
   it.

## Nix calls

Every call is an argv array. Never pass a shell string to `sh -c`.

```text
nix flake update <input>... --flake <flake> --output-lock-file <tmp>
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

## Delta parsing

`nix store diff-closures --derivation` prints package lines such as:

```text
zed-editor: 1.24.2 -> 1.25.2, +12.3 MiB
libfoo: 1.0, 1.1 -> 1.2
newpkg: ∅ -> 2.0
oldpkg: 1.0 -> ∅
empty: ε -> 1.0
```

Accept both `->` and `→`. Split the left and right version lists on commas.
`∅` and `ε` mean no version. Keep a line only when the before and after version
sets differ. Ignore size-only lines and malformed lines with a warning. Sort
changes by package name.

The result is the build-time derivation graph, not the realized runtime
closure. Both text and JSON say so.

## Cost parsing

`nix build --dry-run` prints headers to stderr:

```text
these 721 derivations will be built:
these 671 paths will be fetched (3.0 GiB download, 8.4 GiB unpacked):
```

Parse the counts and sizes. Support `B`, `KiB`, `MiB`, `GiB`, `TiB` with binary
multipliers. Report zero when a header is absent. Cost is per installable; do
not merge installables, because their closures may overlap.

## JSON

`--format json` prints one object. The committed schema is
`schema/report.schema.json`.

```json
{
  "schemaVersion": 1,
  "flake": "/absolute/path",
  "inputs": [
    {
      "name": "unstable",
      "before": { "rev": "...", "lastModified": 1792094642, "narHash": "..." },
      "after":  { "rev": "...", "lastModified": 1792773339, "narHash": "..." }
    }
  ],
  "installables": [
    {
      "attr": "nixosConfigurations.example.config.system.build.toplevel",
      "before": { "drvPath": "/nix/store/...drv" },
      "after":  { "drvPath": "/nix/store/...drv" },
      "delta": {
        "approximation": "build-graph",
        "changes": [
          { "name": "zed-editor", "kind": "changed",
            "before": ["1.24.2"], "after": ["1.25.2"] }
        ]
      },
      "cost": { "toBuild": 721, "toFetch": 671,
                "downloadBytes": 3221225472, "unpackedBytes": 9019431321 }
    }
  ],
  "attrs": [
    { "attr": "nixosConfigurations.example._module.args.pkgs.unstable.zed-editor",
      "before": "1.24.2", "after": "1.25.2" }
  ],
  "warnings": ["the delta is the build-time derivation graph, not the runtime closure"]
}
```

`delta`, `cost`, `attrs`, and `warnings` are omitted when empty or not
computed. `kind` is `added`, `removed`, or `changed`.

## Text

```text
input unstable
  fe132422 (2026-10-15) -> 3ec63636 (2026-10-23)
installable nixosConfigurations.example.config.system.build.toplevel
  delta (build graph): 449 changes
    zed-editor 1.24.2 -> 1.25.2
  cost: 721 to build, 671 to fetch (3.0 GiB download, 8.4 GiB unpacked)
warning: the delta is the build-time derivation graph, not the runtime closure
```

Text lists every parsed change. `--format json` is the scripting surface.

## Exit codes

- `0` success, no `--exit-code` change.
- `1` success with `--exit-code` when an input changed, a package changed, or
  `toBuild + toFetch > 0`.
- `2` usage or Nix error.

## Testing

A fake `nix` executable is injected with `--nix-bin`. It records argv in a log
and prints canned stdout and stderr. Tests assert the exact argv, so a missing
`--dry-run` or `--derivation` fails the suite.

Fixture locks cover input comparison. Captured real output covers the delta and
cost parsers. A real-Nix test is `#[ignore]` and runs only when a local fixture
flake is supplied.

## Out of scope

- Applying the update or switching the system.
- Remote flake URLs in the MVP.
- Exact runtime closures without realized outputs.
- Cache walking, vulnerability checks, or a replacement for `nix-why-build`.
