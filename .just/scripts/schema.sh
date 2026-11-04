#!/usr/bin/env bash
set -eo pipefail

out=$(mktemp)
trap 'rm -f "$out"' EXIT
cargo run --quiet -- --print-schema > "$out"
mv "$out" schema/report.schema.json
