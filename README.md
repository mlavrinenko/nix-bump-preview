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
  delta (build graph): 449 changes
    zed-editor 1.24.2 -> 1.25.2
  cost: 721 to build, 671 to fetch (3.0 GiB download, 8.4 GiB unpacked)
warning: the delta is the build-time derivation graph, not the runtime closure
```

## Status

The project is scaffolded. The MVP work is tracked in
[`backlog/tasks/mvp/`](backlog/tasks/mvp/). The product and process goals are in
[`docs/goals.md`](docs/goals.md), the command and safety contract is in
[`docs/design.md`](docs/design.md), and the report contract is in
[`docs/report.md`](docs/report.md).

## How it works

1. Resolve a proposed lock with `nix flake update --output-lock-file`.
2. Evaluate the old and proposed system derivations with `nix path-info
   --derivation`.
3. Diff the two derivation graphs with `nix store diff-closures --derivation`.
4. Ask Nix what the proposed lock would build or fetch with `nix build
   --dry-run`.
5. Print text or JSON. Nothing is realized, switched, or activated.

`--lock-file` skips step 1 and previews a lock you already resolved.

## JSON

`--format json` is the scripting surface. It is schema-backed at
[`schema/report.schema.json`](schema/report.schema.json).

```bash
nix-bump-preview --input unstable \
  --installable nixosConfigurations.example.config.system.build.toplevel \
  --format json |
  jq '.installables[] | {attr, changes: .delta.changes, cost}'
```

## Limits

The version delta comes from the build-time derivation graph. It over-reports
build-only inputs, and it cannot show the exact runtime closure without built
or substituted outputs. The report and the README say so; the tool never
pretends the approximation is exact.

## Install

```bash
cargo install nix-bump-preview
```

Or download a binary from the
[latest release](https://github.com/mlavrinenko/nix-bump-preview/releases/latest).

## Development

Prerequisites: [Nix](https://nixos.org/) with flakes enabled.

```bash
direnv allow         # or: nix develop
just install-hooks   # pre-commit hook running the (memoized) gate

just check           # fmt, clippy, tests, unused deps, file size, tasks, drift, copy-paste
just deny            # cargo-deny: advisories, licenses, bans (needs network)
just build
just test
just cover           # code coverage via cargo-llvm-cov (70% minimum)
just fmt             # format code
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions.

## License

MIT
