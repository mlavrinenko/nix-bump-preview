#!/usr/bin/env bash
set -eo pipefail

if [ -n "${CI:-}" ]; then
    echo "check-tasks: skipped on CI, which has no MindTape"
elif ! command -v mt >/dev/null; then
    echo "check-tasks: skipped, no MindTape on PATH" >&2
else
    mt check -q
fi
