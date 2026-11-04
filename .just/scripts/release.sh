#!/usr/bin/env bash
set -euo pipefail

version=${1:?usage: release.sh VERSION}
args=(--no-confirm)
if [ -z "${DRY_RUN:-}" ]; then
    args+=(--execute)
fi
cargo release "$version" "${args[@]}" --no-tag
if [ -n "${DRY_RUN:-}" ]; then
    echo "dry run: every check passed"
    echo "would push main, wait for ci.yml on it, then tag and push 'v$version'"
    exit 0
fi
.just/scripts/release-ci.sh "$version"
cargo release tag "${args[@]}"
cargo release push "${args[@]}" \
    || {
        git tag -d "v$version" >/dev/null
        echo "error: origin rejected the push of 'v$version'" >&2
        echo "hint: re-run just release $version" >&2
        exit 1
    }
workflow=$(basename "$(ls .github/workflows/release*.yml | head -1)")
echo "pushed 'v$version'; follow it with: gh run list --workflow $workflow --limit 1"
