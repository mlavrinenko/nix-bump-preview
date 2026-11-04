#!/usr/bin/env bash
set -eo pipefail

# Empty stdout means the lcov can be scored against this tree. Any other line
# names why not. Always exits 0 because the reason is the channel.
lcov=target/coverage/lcov.info
if [ ! -f "$lcov" ]; then
    echo "$lcov is missing"
    exit 0
fi

dirs=()
for d in src tests crates; do
    [ ! -d "$d" ] || dirs+=("$d")
done
if [ ${#dirs[@]} -eq 0 ]; then
    exit 0
fi
newer=$(find "${dirs[@]}" -name '*.rs' -newer "$lcov" -print -quit)
if [ -n "$newer" ]; then
    echo "$lcov predates $newer"
fi
