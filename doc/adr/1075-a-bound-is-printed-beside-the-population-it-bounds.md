# ADR 1075 — A bound is printed beside the population it bounds, and the slack between them is gated

## Status

Accepted, 2026-09-14. Session 1061, the instruments slot.
Crate: `crates/gate-ratchet`. Check: `tools/conformance/tests/ratchets.rs` (tier 1).
`§N` is ISO 32000-2 and nothing else; a section of this document is named in words.

## Context

A ratchet in this tree is a number written by hand into a gate: a **ceiling** a population may only
fall below, or a **floor** it may only rise above. The population comes off the run. The bound comes
off the source. Until now the two never met on one line, and that is the whole of the defect.

Twice in one run of sessions a bound was found sitting well clear of what it bounds:

- **`MAX_INCOMPLETE` at 91 against a population of 61** (session 1036). It had stood at the
  hundred-and-twenty-seventh session's number while nine hundred sessions of work took the
  population down under it. Thirty documents could have started drawing incompletely and the gate
  would not have spoken — a third of its own headline, silently.
- **`MAX_PAGELESS` at 6 against 5** (session 1054). One document left the population when §7.3.7's
  entries-whole reading gave it a page, and the bound stayed where the arrival before it had put it.

Neither was hidden. Both gates print their population on every run; neither printed its bound. The
two numbers were a `grep` and a comparison apart, and putting them side by side was nobody's job
until somebody did it.

The second half is what makes this an ADR rather than two commits. Session 1054 then checked the
other four bounds **in that one file** by hand, and stopped there. A check done by hand is done
once: nothing looked at the other gate files, and nothing would look at that one again.

## Decision

**Every hand-written bound in a gate goes through one crate, which prints it beside its population
on every run and fails when the distance between them exceeds what the bound's own comment allows.**

Three parts, and each answers one half of the failure above.

### 1. `crates/gate-ratchet` — the bound and the population on one line

Four functions: `ceiling`, `floor`, `ceiling_with_headroom`, `floor_with_headroom`. Each prints

```
ratchet: <what>: <population>, ceiling <bound>, slack <slack>
```

and then asserts twice, in this order: the **bound holds** (the assertion the gate already made,
unchanged and unweakened), and the **slack is within what is allowed**.

The plain `ceiling` and `floor` allow none. A bound is the population, and the round that moves the
population moves the bound in the same commit with its reason above the constant. That is not
tidiness: a ceiling *n* above its population is exactly *n* of regression it would admit without
speaking, and a floor *n* below its population is *n* of capability that could be lost the same way.

The printing comes first so the two numbers are on the run's output even when what follows fails.

### 2. Headroom is argued, in the source, and printed

`ceiling_with_headroom` and `floor_with_headroom` take a distance and the argument for it, and print
both. The distinction they draw is real: zero slack is right for a population *this tree alone*
decides — how many corpus documents draw incompletely is a fact about this program and a pinned
submodule — and it is wrong for a population another program's version decides, where a reference
upgrade would fail a gate that then gets loosened in a hurry.

What headroom is never for is "the run is inconvenient". A bound whose only reason is that nobody
re-measured is the defect above wearing a justification.

### 3. `tools/conformance/tests/ratchets.rs` — the part that is not done by hand

A tier-1 check, because the gates it is about are tier-2 and tier-3 walks and a check that only ran
when one of those ran could not see a bound added beside them. It derives its population from
`doc/todo/02` §2's own fenced blocks — the document that **owns** the gate sequence — and holds every
gate file to two rules:

1. An integer `const` named `MAX_…`, `MIN_…`, `…_FLOOR` or `…_CEILING` appears in code only inside a
   `gate_ratchet` call. A bare `assert!(count <= MAX_…)` is what hid both defects.
2. A gate file does not write its own `floor` or `ceiling`. Two of them did, printing nothing, and
   one of those held twenty-eight bounds.

A `// not a ratchet: <reason>` comment above a constant admits a recursion depth or a print limit,
and is checked in both directions: an excuse admitting nothing has outlived its fact.

**The derived population is the part that paid immediately.** This round was commissioned with a
hand-written list of nine gate files. §2 named three more that carry bounds — `pdf-model`'s `dates`
and `xmp` gates among them, with five ratchets between them that the list did not mention. That is
trap 25, and it is why the list is not in the check.

## Consequences

- A round that moves a population now moves its bound in the same commit, or the gate says so. The
  cost is one line per moved population, which is what every entry in `corpus.rs`'s comments already
  is.
- A round that *widens* a population deliberately — trap 5's rise-on-purpose — pays the same line. It
  was already supposed to.
- What the check cannot see is stated in its own module comment rather than papered over: a bound
  whose name says nothing, compared by hand. The naming convention is the handle, and rule 2 narrows
  what a round would have to do to get past it.
- The bands in `doc/checks/launch-path.toml` are deliberately outside this. A band is two-sided, so
  a figure cannot drift within it the way a population drifts under a one-sided ceiling, and the
  bands are recalibrated by a measuring round on a stated machine. What is still true of them is
  that a figure's value is printed only when it is *outside* its band; putting it there on every run
  is worth doing and needs the round that can run that gate's display.
