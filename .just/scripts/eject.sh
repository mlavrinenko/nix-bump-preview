#!/usr/bin/env bash
set -eo pipefail

pct=${1:-90}
root=${2:-src}

# linecop exits 1 to flag files at the baseline, which is a finding, not a
# failed run. Exit 2 and above still fail.
rc=0
paths="$(linecop --baseline "$pct" --format paths)" || rc=$?
[ "$rc" -le 1 ] || exit "$rc"

# ejectest moves inline Rust test modules, so a near-limit Markdown file must
# never reach it.
rust_paths="$(printf '%s\n' "$paths" | grep '\.rs$' || true)"
if [ -n "$rust_paths" ]; then
    printf '%s\n' "$rust_paths" | ejectest apply "$root" --files-from - --lenient
fi
