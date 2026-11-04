#!/usr/bin/env bash
set -eo pipefail

diff_file=$(mktemp)
trap 'rm -f "$diff_file"' EXIT
git diff "$(git merge-base main HEAD)" > "$diff_file"
CARGO_MUTANTS_JOBS="${CARGO_MUTANTS_JOBS:-3}" \
    cargo mutants --in-diff "$diff_file" "$@"
