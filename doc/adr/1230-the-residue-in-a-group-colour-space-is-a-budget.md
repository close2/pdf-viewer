# 1230 — The residue in §11.7.2 is a budget, and half of what the row named was not one

Status: accepted. Session 1196.
Context: ISO 32000-2 §11.7.2, §11.6.6, §11.4.7, §8.6.5.6, §14.11.5; ADRs 0262, 0263, 0417, 0796,
0797, 1056, 1103, 1157, 1193, 1207.
Code: `crates/pdf-model/src/content/transparency.rs` (read, not changed).
Tests: `crates/pdf-model/tests/transparency_groups.rs::a_group_that_introduces_a_press_composites_in_it`.

## 1. What the row said

§11.7.2's ledger row ended: "What keeps this row `partial` is the four components with no profile
behind them and the press past `MAX_PRESSES` … such a group has no press to composite in, so its
elements are painted in the parent's space."

Read against `content/transparency.rs` after ADRs 1157, 1193 and 1207 — the three rounds that have
since moved colour under this clause — the first half is false on the reading it invites and the
second half is a budget rather than a debt.

## 2. The first half

"Four components with no profile behind them" reads as a `/DeviceCMYK` group or page with no ICC
profile anywhere behind it, and that case **composites in ink**. `press_for_entry` sends a
`ColourSpace::Cmyk` entry to `named_press`, which ranks §8.6.5.6's `/DefaultCMYK` above §14.11.5's
intent above ADR 0263's assumed inks, and answers `PagePress::In(assumed_press())` where none of
the three names a profile. The row's own earlier paragraphs say exactly this; the last sentence
contradicted them, which is `doc/todo/01`'s sixth failure shape — a correction appended without the
sentence below it being re-read.

What `press_for_entry` actually refuses is the other arm: a `/CS` of four components that is
neither `DeviceCMYK` nor a four-channel `ICCBased` space — "a `DeviceN` of four inks, say", as its
own comment puts it. **That is not a debt either**, and §11.6.6 is why. The restrictions it places
on a group colour space

> exclude Lab and lightness-chromaticity ICCBased colour spaces, as well as the special colour
> spaces Pattern , Indexed , Separation , and DeviceN

so a four-component space reaching that arm is one the clause does not admit as a group colour
space at all. Refusing it with a report is the clause carried out. (ADR 1229's `Simulated` spaces
are `DeviceN` spaces and fall under the same exclusion, so the separation-simulation preference
adds nothing here.)

## 3. The second half, which is what is left

`MAX_PRESSES` is a bound on how many distinct presses one page may sample, refused per page since
ADR 0417 so that the refusal is the page's property and not the scheduler's. A page past it is
painted in its parent's space rather than in the space §11.7.2 names, and that is a requirement not
executed — so the row stays `partial`, on one sentence instead of two, and the sentence now names
the bound rather than a colour space.

It is a resource bound under `CLAUDE.md` principle 3 rather than a gap in the reading, and trap 38
is what it owes next: whether the standard, or data the standard requires a reader to carry, states
a number this bound must be at least as large as. Nothing in §11.7.2 does. That is a question for a
round that measures how many presses real pages name, which this one did not.

## 4. What this ADR does not do

It changes no code. A row whose last sentence is half false and half a budget is a claim about this
tree, and correcting it is the whole of the work — `CLAUDE.md`'s "a fact that can be counted is not
written down" has a twin that this is an instance of: a sentence that *can* be checked against the
tree goes stale silently, and only reading the two together finds it.
