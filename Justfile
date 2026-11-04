set quiet
set shell := ["bash", "-eo", "pipefail", "-c"]

rust_roots := if path_exists("crates") == "true" { "crates" } else { "src tests" }
eject_root := if path_exists("crates") == "true" { "crates" } else { "src" }

[doc("List available recipes")]
[group("workflow")]
default:
    @just --list

[doc("Run fixes, then the full gate")]
[group("workflow")]
fix-check: eject clippy-fix fmt check

[doc("Run every gate, each memoized by mmz")]
[group("workflow")]
[parallel]
check: (mmz "fmt-check") (mmz "clippy") (mmz "test") (mmz "machete") (mmz "check-file-size") (mmz "check-dry") (mmz "check-tasks") (mmz "outdatty-check")

[doc("Run a recipe through mmz memoization")]
[group("workflow")]
[no-exit-message]
mmz *ARGS:
    mmz just {{ ARGS }}

[doc("Show each gate's freshness")]
[group("workflow")]
status:
    mmz --status

[doc("Check the task backlog")]
[group("gate")]
check-tasks:
    bash .just/scripts/check-tasks.sh

[doc("Fail if a source changed without its dependents re-confirmed")]
[group("gate")]
outdatty-check:
    outdatty check

[doc("Re-confirm dependency groups and refresh the lock")]
[group("dev")]
outdatty-update:
    outdatty update

[doc("Run the test suite and doctests")]
[group("gate")]
[positional-arguments]
test *ARGS:
    bash .just/scripts/test.sh "$@"

[doc("Run clippy only")]
[group("gate")]
clippy:
    cargo clippy --workspace --all-targets -q -- -D warnings

[doc("Auto-fix clippy warnings")]
[group("dev")]
clippy-fix:
    cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged -- -D warnings

[doc("Build the project")]
[group("dev")]
[positional-arguments]
build *ARGS:
    cargo build --workspace -q "$@"

[doc("Regenerate the committed report schema")]
[group("dev")]
schema:
    bash .just/scripts/schema.sh

[doc("Run coverage with cargo-llvm-cov")]
[group("gate")]
cover:
    bash .just/scripts/cover.sh

[doc("Gate complex, undertested functions via CRAP")]
[group("gate")]
crap:
    bash .just/scripts/crap.sh

[doc("Format code")]
[group("dev")]
fmt:
    cargo fmt --all

[doc("Check formatting without writing changes")]
[group("gate")]
fmt-check:
    cargo fmt --all -- --check

[doc("Check for unused dependencies")]
[group("gate")]
machete:
    out=$(cargo machete 2>&1) || { printf '%s\n' "$out"; exit 1; }

[doc("Count tests across the workspace")]
[group("advisory")]
count-tests:
    cargo nextest list --workspace --message-format json \
        | jq -r '[.["rust-suites"][].testcases[] | select(.ignored == false)] | "\(length) tests"'

[doc("Show the top 20 files by line count")]
[group("advisory")]
file-sizes:
    find . -type f \( -name '*.rs' -o -name '*.md' \) ! -path './target/*' -exec wc -l {} + | sort -rn | awk 'NR <= 20'

[doc("Eject inline tests from Rust files nearing the linecop cap")]
[group("dev")]
eject PCT='90':
    bash .just/scripts/eject.sh {{ PCT }} {{ eject_root }}

[doc("Fail on oversized files")]
[group("gate")]
check-file-size:
    linecop

[doc("Fail on duplicated Rust blocks")]
[group("gate")]
check-dry:
    jscpd --no-tips -k 70 -f rust --exit-code 1 -r ai {{ rust_roots }}

[doc("Audit advisories, licenses and dependency bans")]
[group("advisory")]
deny:
    cargo deny -L error check --hide-inclusion-graph advisories licenses bans

[doc("List dependencies with newer upstream versions")]
[group("advisory")]
outdated:
    cargo outdated --workspace

[doc("Fork a sibling worktree on a new branch")]
[group("workflow")]
[positional-arguments]
fork SLUG *ARGS:
    wt switch --create --yes "$@"

[doc("Gate, fast-forward the branch onto main and remove its worktree")]
[group("workflow")]
[positional-arguments]
land *ARGS:
    wt merge --no-squash --no-commit --yes "$@"

[doc("Replay two parallel landing lanes in a scratch repo")]
[group("workflow")]
land-repro:
    bash .just/scripts/land-repro.sh

[doc("Point git at the tracked hooks and merge drivers")]
[group("dev")]
install-hooks:
    bash .just/scripts/install-hooks.sh

[arg('VERSION', pattern='\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?')]
[arg('DRY_RUN', long='dry-run', value='true')]
[doc("Push main, wait for its CI, then tag and push the tag")]
[group("release")]
[no-exit-message]
release VERSION $DRY_RUN='':
    DRY_RUN="{{ DRY_RUN }}" bash .just/scripts/release.sh {{ VERSION }}

[doc("Sweep mutants touched by this branch's diff")]
[group("advisory")]
[positional-arguments]
mutants-diff *ARGS:
    bash .just/scripts/mutants-diff.sh "$@"

[doc("Run a full-file mutation sweep")]
[group("advisory")]
[positional-arguments]
mutants *ARGS:
    cargo mutants "$@"
