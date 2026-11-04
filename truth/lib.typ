// The goal check every open task imports. `check-goals(value)` checks a task's
// `x-goals` field against both goal sets, goals.typ (what the tool is for) and
// process-goals.typ (how this repo builds it), returning one message per
// problem instead of panicking: an `mt` rule collects every string it returns
// as a breach, so several omissions surface together rather than one at a
// time. A task nothing justifies (helps no goal) is refused, not filed.

#import "goals.typ": goals as product-goals
#import "process-goals.typ": goals as process-goals

// A task rates a goal with one of these, or with a plain effect string.
#let helps(reason) = (effect: "helps", reason: reason)
#let hurts(reason) = (effect: "hurts", reason: reason)
#let neutral = (effect: "neutral")

// Fill the goals a task does not mention with `neutral`. The explicit
// overrides win, so a goal named in both lists keeps its verdict.
#let verdicts(overrides, ..neutral-keys) = {
  let result = (:)
  for (key, value) in overrides {
    result.insert(key, value)
  }
  for key in neutral-keys.pos() {
    if key not in result {
      result.insert(key, neutral)
    }
  }
  result
}

#let _effect(value) = if type(value) == str {
  (value, none)
} else if type(value) == dictionary {
  (value.at("effect", default: none), value.at("reason", default: none))
} else {
  (none, none)
}

// One group's worth of breaches ("product" or "process"): every registry goal
// not addressed, every addressed key the registry does not recognize, every
// value that is not helps/hurts/neutral, and every helps/hurts with no reason.
// Returns (messages, effects) so the caller can tell whether anything helps.
#let _check-group(group, registry, given) = {
  if given == none {
    return (
      (
        group
          + " is missing from x-goals; add "
          + group
          + ": (...), one verdict per goal",
      ),
      (),
    )
  }
  if type(given) != dictionary {
    return ((group + " takes a dictionary of goal verdicts",), ())
  }
  let messages = ()
  let effects = ()
  for key in registry.keys() {
    if key not in given {
      messages.push(
        group
          + " goal `"
          + key
          + "` is not addressed; say helps[...], hurts[...] or neutral",
      )
    }
  }
  for (key, value) in given {
    if key not in registry {
      messages.push(
        "unknown "
          + group
          + " goal `"
          + key
          + "`; they are "
          + registry.keys().join(", "),
      )
      continue
    }
    let (effect, reason) = _effect(value)
    if effect not in ("helps", "hurts", "neutral") {
      messages.push(
        group + " goal `" + key + "` takes helps[...], hurts[...] or neutral",
      )
      continue
    }
    effects.push(effect)
    if (
      effect != "neutral"
        and (reason == none or reason == "" or reason == [])
    ) {
      messages.push(group + " goal `" + key + "` says " + effect + " but not how")
    }
  }
  (messages, effects)
}

// `[[tasks.rules]] check` entry point for `field = "x-goals"`: `value` is
// `none` when the task carries no `x-goals` at all. Returns an array of breach
// strings — empty means the task passes.
#let check-goals(value) = {
  if value == none or type(value) != dictionary {
    return (
      "x-goals is missing; add x-goals: (product: (...), process: (...))",
    )
  }
  let (product-messages, product-effects) = _check-group(
    "product",
    product-goals,
    value.at("product", default: none),
  )
  let (process-messages, process-effects) = _check-group(
    "process",
    process-goals,
    value.at("process", default: none),
  )
  let messages = product-messages + process-messages
  let effects = product-effects + process-effects
  if effects.len() > 0 and effects.all(effect => effect != "helps") {
    messages.push("the task helps no goal; if nothing justifies it, cancel it")
  }
  messages
}
