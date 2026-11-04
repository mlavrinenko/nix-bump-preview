# Report contract

This document is the source of truth for the delta, cost, JSON, text, exit
codes, and tests. The command, sequence, Nix calls, and safety rules are in
[design.md](design.md).

## Delta parsing

`nix store diff-closures --derivation <before.drv> <after.drv>` prints package
lines such as:

```text
zed-editor: 1.24.2 -> 1.25.2, +12.3 MiB
libfoo: 1.0, 1.1 -> 1.2
newpkg: ∅ -> 2.0
oldpkg: 1.0 -> ∅
empty: ε -> 1.0
```

Accept both `->` and `→`. Split the left and right version lists on commas.
`∅` and `ε` mean no version. Group by package name, union the before and after
version sets, and keep one entry when the sets differ. A size-only line is
skipped without a warning because it is not a version change. A malformed line
appends exactly `warning: ignored malformed diff line: <line>` to the top-level
warnings, in input order. The parser never panics.

Changes are sorted by package name and classified as `added`, `removed`, or
`changed`. The result is the build-time derivation graph, not the realized
runtime closure. Both text and JSON say so.

## Cost parsing

`nix build --dry-run` prints headers to stderr:

```text
these 721 derivations will be built:
these 671 paths will be fetched (3.0 GiB download, 8.4 GiB unpacked):
```

Parse the counts and sizes. Support `B`, `KiB`, `MiB`, `GiB`, and `TiB` with
binary multipliers. A fractional size such as `3.5 KiB` is parsed as a decimal
and rounded half up to whole bytes. An overflow beyond `u64` is an error and
exits 2. A missing header is zero. Singular and plural headers both work. Cost
is per installable and is not merged across installables, because their
closures may overlap.

Text prints `X.Y` with one decimal in binary units, trimming a trailing `.0`
and using the largest unit that keeps the value at or above 1. JSON stores
whole bytes.

## JSON

`--format json` prints one pretty-printed object with struct-order keys and
camelCase names. The committed schema is `schema/report.schema.json`.

```json
{
  "schemaVersion": 1,
  "flake": "/absolute/path",
  "inputs": [
    {
      "name": "unstable",
      "changed": true,
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
computed. `before` and `after` on an input omit `lastModified` or `narHash` when
the lock lacks them. `kind` is `added`, `removed`, or `changed`. Arrays preserve
CLI order except `inputs` and `delta.changes`, which are sorted by name.

## Text

```text
input unstable
  fe132422 (2026-10-15) -> 3ec63636 (2026-10-23)
input nixpkgs: unchanged
installable nixosConfigurations.example.config.system.build.toplevel
  delta (build graph): 449 changes
    zed-editor 1.24.2 -> 1.25.2
    newpkg ∅ -> 2.0
    oldpkg 1.0 -> ∅
  attr nixosConfigurations.example._module.args.pkgs.unstable.zed-editor: 1.24.2 -> 1.25.2
  cost: 721 to build, 671 to fetch (3.0 GiB download, 8.4 GiB unpacked)
warning: the delta is the build-time derivation graph, not the runtime closure
```

Revisions are truncated to eight characters. Dates are UTC `YYYY-MM-DD` from
`lastModified`; when it is absent, print the revision only. Multiple versions
are comma-separated. Text prints every parsed change. `--format json` is the
scripting surface.

## Exit codes

- `0` success, no `--exit-code` change.
- `1` success with `--exit-code` when a selected input changed, a package
  changed, or the computed cost has `toBuild + toFetch > 0`.
- `2` usage, validation, or Nix error.

With `--no-delta`, the package-change part is not evaluated. With `--no-cost`,
the cost part is not evaluated. `--exit-code` only considers what was computed.

## Testing

A fake `nix` executable is injected with `--nix-bin`. It records argv in a log
and prints canned stdout and stderr. Tests assert the exact argv, so a missing
`--dry-run` or `--derivation` fails the suite.

Fixture locks cover a changed input, an unchanged input, `--all`, and
`--lock-file`. The fake covers two unrelated flakes and installables, so
`generic` is exercised. A full success and a full failure both assert that the
before `flake.lock` keeps its bytes and mtime. `--no-delta` and `--no-cost`
tests assert the corresponding Nix call is absent from the argv log.

Captured real output is committed under `tests/fixtures/captured/` for the
delta and cost parsers. A real-Nix test is `#[ignore]` and runs only when
`NIX_BMP_REAL_FLAKE` points at a local fixture flake. `just check` runs no
network and no real Nix.
