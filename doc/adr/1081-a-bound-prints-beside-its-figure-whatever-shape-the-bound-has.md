# ADR 1081 — A bound prints beside its figure on every run, whatever shape the bound has

## Status

Accepted, 2026-09-15. Session 1067, the instruments slot. Extends ADR 1075, which this does not
amend: everything it decided stands, and two of the three things it left undone are done here.
Crates: `crates/gate-ratchet`. Gates: `pdf-model --test corpus`, `--test fixed_documents`,
`viewer-ui --test launch_path`. Section: `tools/state.sh ratchets`.
`§N` is ISO 32000-2 and nothing else; a section of this document is named in words.

## Context

ADR 1075 put every one-sided bound beside the population it bounds and gated the slack between
them. It left three things standing, each stated in its own Consequences:

- **The three ceilings in `corpus.rs` whose members already had names.** `MAX_LOCKED` (10),
  `MAX_PAGELESS` (5) and `MAX_UNREADABLE_ENCRYPTION` (1) each named every member in a doc comment,
  and `tally.locked` was already a `Vec` of those names — but converting one changes what the gate
  *asserts*, which that round was told not to do.
- **The two-sided bands.** `doc/checks/launch-path.toml`'s clock, memory, byte and instruction
  bands and `doc/checks/fixed-documents.toml`'s ink bands print a figure's value **only when it is
  outside**, which ADR 1075 called "worth doing" and deferred to "the round that can run that
  gate's display".
- **One command for the whole table.** Nothing ran every gate that carries a bound.

The deferral of the bands rested on a premise that is false: `viewer-ui --test launch_path` runs
under `--release` in tier 2, headless, with no display — rounds 1027 and 1057 both ran it.

## Decision

**A bound prints beside its figure on every run, whatever shape the bound has**, and
`crates/gate-ratchet` is the one place that line is written. Two entry points join the four:

### 1. `population(what, found, named)` — where the members have names, the names are the bound

It prints the same line as `ceiling` (the count beside the length of the list, so the table stays
one table) and then holds the two sets equal **in both directions**, naming what joined and what
left. The argument is one sentence: a count of ten cannot tell a document that *started* needing a
password from one that *stopped*, and both are findings. Measured rather than asserted — swapping
one name in `LOCKED` for one that does not exist leaves the printed line reading `10, ceiling 10,
slack 0` and fails naming both halves of the swap.

The list carries the reason each member is in it beside its name, which is the second half of the
decision and not decoration: a document leaves such a population when somebody fixes something, and
the round that deletes the name is the round that has to read what the name was for.

### 2. `band(what, value, low, high)` — the figure, the band, and the distance to the nearer edge

A two-sided band has no slack in ADR 1075's sense, because a figure cannot drift *within* one the
way a population drifts under a ceiling. What it has is a distance to each edge, and the smaller of
the two is what says whether the next legitimate change fires the gate. It is printed with the
share of the band's width it represents, so figures in different units are comparable at a glance:
50% is the middle, 0% is the edge.

**It prints and does not assert, and that is the difference between a band and a one-sided bound.**
A band's verdict has conditions this crate has no business knowing — the launch path declines a
figure whose child's probe says the machine was busy; the fixed-documents check pins a *refusal* on
some rows and nothing at all on others. Those verdicts stay in the gates, unweakened and untouched.

### 3. `tools/state.sh ratchets` — the whole table off one run

A composed section: every line it runs is a line another section already runs, with the ratchet
table kept instead of that gate's summary. Its population is **derived** — every tracked test file
under `crates/` or `tools/` that calls into `gate-ratchet` — because a hand-written list is trap
25's shape and ADR 1075's own check found three files carrying bounds nobody had listed. *How* to
run each comes from `doc/todo/02` §2, which owns the sequence, down to the profile and the
`--ignored`.

## Consequences

- Three of `corpus.rs`'s five bounds are now names; the other two (`MAX_UNOPENABLE` at 0,
  `MAX_INCOMPLETE` at 61) stay counts, the first because it has no members and the second because
  its members are a classification the run already prints in full.
- `ratchets.rs`'s own floor on how many bounds it finds routed moves from 10 to 8, with the reason
  above it: a `&[&str]` is not an integer const, so a gate getting *better* takes three off that
  count. The scan deliberately does not read named populations, and says so.
- **The printing paid on its first run.** Three `open_kinstructions` figures sit 10–15% from the
  *high* edge of bands whose own measured spread is 0.011% — `xfa_filled_imm1344e.pdf` has 3.7
  thousand instructions of headroom on 1816 — so the next round that adds a little to the open path
  fails a gate for a change nobody would call a regression. No band moves here: a band is a claim
  about a machine and moving one needs the measurement, not the discovery.
- **A 0% share is not always a warning** (trap 39). Eight `fixed-documents.toml` rows pin a page
  that is honestly blank as `ink = 0.0 .. 1.0`, and ink has nowhere below zero to go, so the figure
  sits on the low edge on every run by design. The *falling* share is the signal; a constant one is
  a fact about the row. Said in `band`'s own doc comment, where the next reader is.
- What ADR 1075 left that is still left: a bound whose name says nothing, compared by hand.
