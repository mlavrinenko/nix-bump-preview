# nix-bump-preview

[![CI](https://github.com/mlavrinenko/nix-bump-preview/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/nix-bump-preview/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/nix-bump-preview.svg)](https://crates.io/crates/nix-bump-preview)
[![License: MIT](https://img.shields.io/crates/l/nix-bump-preview.svg)](LICENSE-MIT)

Preview a Nix flake input bump before applying it. It reports which package
versions would change and what the proposed lock would build or fetch, without
building the target or touching `flake.lock`.

```console
$ nix-bump-preview \
    --input unstable \
    --installable nixosConfigurations.example.config.system.build.toplevel
input unstable
  fe132422 (2026-10-15) -> 3ec63636 (2026-10-23)
installable nixosConfigurations.example.config.system.build.toplevel
  delta (build graph): 3 changes
    libfoo 1.0 -> 1.1
    newpkg ∅ -> 2.0
    zed-editor 1.24.2 -> 1.25.2
  cost: 721 to build, 671 to fetch (3.0 GiB download, 8.4 GiB unpacked)
warning: the delta is the build-time derivation graph, not the runtime closure
```

## Usage

```text
nix-bump-preview --input <name>... --installable <attr>... [options]
```

`--input` selects a root input, `--all` selects every root input, and
`--lock-file` previews a lock you already resolved instead of calling
`nix flake update`. At least one `--installable` is required; `--attr` adds a
direct `.version` comparison. `--flake` defaults to `.` and must be a local
directory containing `flake.lock`.

| Option | Meaning |
| --- | --- |
| `-f, --flake <DIR>` | Local flake directory, default `.` |
| `-i, --input <NAME>` | Root input to update; repeatable |
| `--all` | Update every root input; exclusive with `--input` |
| `-l, --lock-file <PATH>` | Preview an existing proposed lock; selects every root input |
| `--installable <ATTR>` | Attr under the flake to diff; repeatable, required |
| `--attr <ATTR>` | Attr with a `.version` for a direct version delta; repeatable |
| `--format <text\|json>` | Output format, default `text` |
| `--keep-lock <PATH>` | Copy the generated proposed lock to `PATH` |
| `--no-delta` | Skip the package delta and its Nix call |
| `--no-cost` | Skip the build and fetch cost and its Nix call |
| `--exit-code` | Exit 1 when a change or build cost exists, else 0 |

## How it works

1. Resolve a proposed lock with `nix flake update --output-lock-file`.
2. Evaluate the old and proposed system derivations with `nix path-info
   --derivation`.
3. Diff the two derivation graphs with `nix store diff-closures --derivation`.
4. Ask Nix what the proposed lock would build or fetch with `nix build
   --dry-run`.
5. Print text or JSON. Nothing is realized, switched, or activated.

`--lock-file` skips step 1 and previews a lock you already resolved.

The version delta comes from the build-time derivation graph, not the runtime
closure. It over-reports build-only inputs, and an exact runtime closure needs
built or substituted outputs. The report carries that warning.

## JSON

`--format json` is the scripting surface. It is schema-backed at
[`schema/report.schema.json`](schema/report.schema.json). The committed
[example report](tests/fixtures/reports/example.json) shows the shape:

```bash
jq '.installables[] | {attr, changes: .delta.changes, cost}' \
  tests/fixtures/reports/example.json
```

## Install

```console
$ cargo install nix-bump-preview
```

Prebuilt binaries are on the
[latest release](https://github.com/mlavrinenko/nix-bump-preview/releases/latest).

The flake is also an input:

```nix
inputs.nix-bump-preview.url = "github:mlavrinenko/nix-bump-preview";
```

Use `inputs.nix-bump-preview.packages.${pkgs.stdenv.hostPlatform.system}.default`
in `home.packages` or `environment.systemPackages`. To try it without installing:

```console
$ nix run github:mlavrinenko/nix-bump-preview -- --input unstable --installable ...
```

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
