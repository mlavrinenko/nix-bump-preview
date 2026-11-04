# Goals

`nix-bump-preview` answers one question before an update is applied: what would
this flake input bump change, and what would it cost to build or fetch? It
does not apply the update.

Every open task names the product and process goals it serves in its `Goals`
section. A task that helps none is not ready to start.

## Product goals

### `proposed-lock`

Preview a proposed input bump without modifying the user's `flake.lock`.

Acid test: after a run, the lock file's bytes and mtime are unchanged, and
`git status` shows no change caused by the tool. `--lock-file` takes an
already-proposed lock instead of resolving one.

### `package-delta`

Show which package versions change, including changes whose new outputs are
already in the local store.

Acid test: a fixture with a realized new output still reports the version
change. The delta comes from the build-time derivation graph, not from a
store-presence check.

### `build-cost`

Show what the proposed lock would build and fetch, with counts and download and
unpacked bytes.

Acid test: the report distinguishes built from fetched derivations, parses
`B`/`KiB`/`MiB`/`GiB`, and reports zero when there is nothing to do.

### `no-build`

Never realize target outputs. The only side effects are resolving the proposed
lock and reading cache metadata.

Acid test: a fake Nix records every call. No `nix build` without `--dry-run`,
no `nix store diff-closures` without `--derivation`, and no activation command
at all.

### `generic`

Hardcode no flake path, input name, host, system, package set, or installable.

Acid test: integration tests use two unrelated flakes and installables, and a
repository search finds no private hostname, local flake path, OS user name, or
machine-specific package set.

### `machine-readable`

JSON is a first-class output, not a later addition. It carries the same facts
as text and is stable enough for `jq` and CI.

Acid test: `--format json` validates against `schema/report.schema.json`, and
the README shows a `jq` query over the real shape.

### `honest`

Say what the preview cannot know. The version delta is a build-time
approximation; the exact runtime closure needs built or substituted outputs.

Acid test: text and JSON carry a warning naming the approximation, and the
README states the limit.

## Process goals

### `test-first`

Every behavior lands with a failing test first, or the task states why a test
is impossible. Parsers are tested against captured real output and malformed
input.

### `parsers-tested`

Every parser has unit tests for the real format, including Unicode arrows,
empty versions, multiple versions, and truncated lines. A parser never panics
on input.

### `nix-only`

Nix is the only external program. `nvd`, `dix`, `jq`, and shell pipelines are
not runtime dependencies. The tool builds argv arrays and reads Nix's output.

### `no-personal`

No private machine detail appears anywhere: local hostname, local flake path,
OS user name, system name, or a package set specific to one machine. Examples
use placeholders such as `<host>` and `nixosConfigurations.example`. The public
repository owner, repository URL, license attribution, dependency URLs, and
generic placeholder examples are allowed.

### `focused`

The tool orchestrates Nix and formats the result. It does not walk the store,
query binary caches, implement a cache checker, or reimplement a diff engine.

## Non-goals

- Applying, switching, activating, or realizing an update.
- Auditing vulnerabilities or licences.
- Replacing `canibump` (upstream readiness) or `nix-why-build` (why a build
  misses the cache).
- Producing an exact runtime closure from `.drv` files alone.
