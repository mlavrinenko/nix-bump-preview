#!/usr/bin/env bash
set -eo pipefail

# main.rs stays thin and is excluded here as in the CRAP gate. The registry
# ignore keeps a relocated CARGO_HOME from changing the reported file set.
ignore='(^|/)src/main\.rs|/registry/src/'

mkdir -p target/coverage
# The clean is cargo-llvm-cov's precondition for the --no-report split: without
# it, profraw from binaries that no longer exist is merged into the report.
cargo llvm-cov clean --workspace
cargo llvm-cov nextest --workspace --no-report
cargo llvm-cov report --ignore-filename-regex "$ignore" \
    --lcov --output-path target/coverage/lcov.info
cargo llvm-cov report --ignore-filename-regex "$ignore" --fail-under-lines 70
