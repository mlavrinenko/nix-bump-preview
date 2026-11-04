#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: slot(
    "title",
    prompt: "Task title",
    example: "Resolve a proposed lock without writing flake.lock",
  ),
  tags: slot("tags", kind: "list", default: ()),
  priority: slot("priority", kind: "int", default: 5),
)

== Summary

#slot(
  "summary",
  kind: "raw",
  prompt: "The problem, stated once",
  default: "- TODO: what is wrong today, and why it matters",
)

== Goals

#slot(
  "goals",
  kind: "raw",
  prompt: "Product and process goals this task serves",
  default: "- TODO: name the product and process goals this task serves",
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
