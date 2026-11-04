#import "@local/mindtape:0.2.0": *
#import "@local/nix-bump-preview:0.1.0": helps, hurts, neutral, verdicts

#show: task.with(
  title: slot(
    "title",
    prompt: "Task title",
    example: "Resolve a proposed lock without writing flake.lock",
  ),
  tags: slot("tags", kind: "list", default: ()),
  priority: slot("priority", kind: "int", default: 5),
  x-goals: (
    product: verdicts(
      slot(
        "product-goals",
        kind: "raw",
        prompt: "Product goals this task helps or hurts",
        example: "(generic: helps[...],)",
        default: (:),
      ),
      ..slot(
        "product-neutral",
        kind: "list",
        default: (
          "proposed-lock",
          "package-delta",
          "build-cost",
          "no-build",
          "generic",
          "machine-readable",
          "honest",
        ),
      ),
    ),
    process: verdicts(
      slot(
        "process-goals",
        kind: "raw",
        prompt: "Process goals this task helps or hurts",
        example: "(no-personal: helps[...],)",
        default: (:),
      ),
      ..slot(
        "process-neutral",
        kind: "list",
        default: (
          "test-first",
          "parsers-tested",
          "nix-only",
          "no-personal",
          "focused",
        ),
      ),
    ),
  ),
)

== Summary

#slot(
  "summary",
  kind: "raw",
  prompt: "The problem, stated once",
  default: "- TODO: what is wrong today, and why it matters",
)

== Acceptance criteria

#slot(
  "acceptance",
  kind: "raw",
  default: "- TODO: a condition someone else can check without asking you",
)

== Non-goals

#slot(
  "non-goals",
  kind: "raw",
  default: "- TODO: what this task deliberately will not touch",
)

== Files

#slot(
  "files",
  kind: "raw",
  default: "- TODO: paths this task expects to change",
)
