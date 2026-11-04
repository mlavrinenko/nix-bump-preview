#!/usr/bin/env bash
set -eo pipefail

cargo nextest run --workspace --cargo-quiet "$@"
out=$(cargo test --workspace --doc -q "$@" 2>&1) || { printf '%s\n' "$out"; exit 1; }
grep -E '^test result: ok\. [1-9]' <<<"$out" || true
