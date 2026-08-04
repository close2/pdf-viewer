# ADR 0177 — The pointer is on an annotation, not on a link

Status: accepted, two-hundred-and-fifty-third session.
Amends ADR 0123.

## Context

`doc/todo/01`'s capability sweep, run over the ledger, produced §12.5.3's row:

> ReadOnly, Locked, ToggleNoView and LockedContents constrain user interaction, which does not
> exist yet.

Interaction has existed since the hundred-and-thirty-second session. `ReadOnly` turned out to be
implemented already — `annotation::interacts` quotes it verbatim — so the sentence understates
what the code does, which is the sweep's first failure shape. Reading the other three against the
code found something the sweep was not looking for.

**`viewer-core` took the annotation under the pointer from `link_at`**, which returns a
`/Subtype /Link` and nothing else:

```rust
let under = point.and_then(|(x, y)| interact::link_at(open, x, y));
…
PointerAction::Pressed => under.map(|annotation| (annotation, Pointer::Down)),
```

So §12.5.5's three appearances — "[a]n annotation may define as many as three separate
appearances" — reached only links, and §12.5.6.19's `/H` highlighting mode, which is an entry of
a **widget**, could not be reached by any host at all. ADR 0123 built it, `pdf-model` tests it by
pressing a widget directly, and no program could press one. Nothing announced that: the tests pass
because they set the pointer state themselves.

## Decision

**The pointer state follows `annotation_at`, which is every annotation on the page.** That
function is already what §12.6.3's trigger events use, so this is one question asked once rather
than two answers that disagreed. `link_at` stays for what a *click activates*, which is a link's
activation region and only that.

It is `annotation_at` for a second reason, and this one is a clause rather than a tidy-up: that
function filters by `annotation::interacts`, and §12.5.3's `ReadOnly` says an annotation "should
not respond to mouse clicks or change its appearance in response to mouse motions". Reading the
pointer's region through it is what makes that sentence true here.

The per-annotation guard stays exactly as it was — a pointer state is only recorded where the
picture can differ, because changing it invalidates the page's display list and costs 2 000 M
instructions on the benchmark page. What changed is that the guard is now asked about every
annotation instead of about links.

## What it exposed, in the same session

**§12.5.6.19's `/H` had a default reaching subtypes whose clause states no such entry.**
`highlight` returned `Invert` for any annotation with no `/H` and no `/D`, and Table 192 gives
that default to a *widget*; Table 176 gives the same default to a *link*. No other table in the
standard defines `/H`. While the pointer only ever landed on links the overreach could not show
itself; the first test written after this change — a `Square` with `ToggleNoView`, about a
different flag entirely — came back cyan.

So `highlight` now answers `Highlight::None` for a subtype outside `Link` and `Widget`. **A
default belongs to the entry, not to annotations in general.**

## And the flag that was never reachable: `ToggleNoView`

Table 167, bit 9:

> If set, invert the interpretation of the NoView flag for annotation selection and mouse
> hovering, causing the annotation to be visible when the mouse pointer hovers over the
> annotation or when the annotation is selected.

It is a pointer-dependent *reading* of `NoView` rather than a second suppression, so the code is
an exclusive-or: `NoView` alone hides, the two together hide until the cursor arrives, and
`ToggleNoView` alone hides only while it is there. Table 170's appearance is what "hovering" means
here — §12.5.5 defines the rollover as the cursor in the active area without a button and the down
appearance as one held there — so anything but `Appearance::Normal` is the condition this clause
states in prose and that one states in a table.

**One derivation was needed and it is worth stating as one.** `annotation_at` filters by
`interacts`, which returned `false` for a `NoView` annotation — so an annotation carrying both
flags could never be hovered, could never leave `Appearance::Normal`, and bit 9 could not mean
anything at all. The clause states the pair's effect as "causing the annotation to be visible when
the mouse pointer hovers over the annotation", an effect conditioned on the hover being *noticed*;
what it asks for is an annotation that appears under the cursor, which is an annotation whose
region is live. So `NoView` suppresses interaction only where `ToggleNoView` is clear.

**No corpus document states bit 9** — a scan of every uncompressed `/F` in all 974 found none — so
the test is a hand-built fixture and says so. The flag was never blocked on a file.

## The lesson

**A capability can arrive at the crate that implements a clause and never reach the program.**
This project's standing sweep asks what a row's stale *reason* was; the three findings here came
from asking the opposite question — the model implements this, so who calls it? §12.5.6.19 was
`implemented` in the ledger, tested with pixels, argued in an ADR, and unreachable from the only
host in the tree for a hundred and fifteen sessions.

And **widening a region turns a latent default into a wrong pixel**. `/H`'s default was harmless
for exactly as long as the caller was narrower than the code it called. Two of this round's three
findings are that same shape one step apart, which is the argument for running the whole test
suite after a change to *who* is asked rather than to *what* is answered.
