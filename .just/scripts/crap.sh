#!/usr/bin/env bash
set -eo pipefail

reason=$(bash .just/scripts/lcov-stale-reason.sh)
if [ -n "$reason" ]; then
    echo "just crap: $reason — run \`just cover\` first" >&2
    echo "  (a source the lcov never saw scores as 0% covered, not as unknown)" >&2
    exit 1
fi

# github format prints one line per failing function and nothing on a pass.
cargo crap --lcov target/coverage/lcov.info --workspace \
    --exclude '**/main.rs' --exclude 'tests/**' --exclude '**/*_tests.rs' \
    --threshold 30 --fail-above --format github
