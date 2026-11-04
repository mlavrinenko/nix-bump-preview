#!/usr/bin/env bash
set -eo pipefail

git config core.hooksPath .githooks
git config merge.outdatty.name "keep the target's outdatty.lock"
git config merge.outdatty.driver true
rm -f .git/hooks/pre-commit
chmod +x .githooks/pre-commit
echo "installed .githooks"
