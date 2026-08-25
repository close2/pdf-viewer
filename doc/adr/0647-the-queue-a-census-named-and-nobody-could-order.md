# 0647 — The queue a census named, and nobody could order

**Status.** Accepted. Session 744.

Takes what ADR 0643 left owed — *the three-measure "we are alone" list has never been read as a
list* — and reads it, by giving the gate the one number it was missing. **No pixel moves, no
verdict moves and no list changes**: the run before this round and the run after it are identical
on every one of the 962 non-agreeing per-page lines and on every census figure, and the whole diff
under `crates/` is `tests/oracle.rs`, a test target that no library links.

## Context

`doc/todo/00` step 1 asks a round to prefer *the shape that says we are alone* — our distance from
the **nearest** reference larger than the distance between the **closest two** references. Session
518 took that reading by hand, in levels of 255, over 786 ambiguous pages, and recorded 56 of them.

The gate could not take it at all until ADR 0643, because its two columns were two instruments: the
pair's number over four measures, ours over three. That round put ours in both units and printed a
count beneath the ranking — necessarily in the **four**-measure unit, because that was the only one
the pair's number existed in. Read that way the shape names **569 of the 804** complete ambiguous
pages, seven in ten, which is `doc/todo/12`'s bound arriving as a signal rather than a signal: the
differing fraction is a bound the references miss by nearly as much as we do, and ADR 0643 measured
the medians that say so (ours 2.08 times the class bound, the closest pair's 1.96).

So the census named a population nobody could open, in the unit that names most of the bucket, and
the unit that reproduces the hand-taken reading had no printed count and no order at all.

## The decision

**Give the closest pair a number in `Distance`'s unit, and print the queue ordered by the ratio the
two now form.**

- `outside_by_in_three_measures` is the arithmetic `Distance::of` carried inline, extracted so that
  one implementation answers both sides of the comparison. One property it does *not* share with
  `outside_by` is written down where it is defined: on an ambiguous page `outside_by` is above 1 for
  every pair by construction, and this one **can be below 1**, exactly where the closest pair misses
  on the differing fraction alone.
- `consensus_missed_in_three_measures` is `consensus_missed_by` reduced by that function instead.
- `rank_the_pages_we_are_alone_on` prints both counts, the ten largest ratios, and — because the
  ranking cannot be read honestly without them — how many of the list have a denominator below 1 and
  how many have *both* numbers below 1.

`Distance` itself is untouched, for ADR 0643's reason: its figures are a published unit.

## What the corrected instrument says

**The three-measure reading names 48 of the 804**, which is ADR 0643's predicted figure reproduced
by the gate that now prints it, and 6% of the pool against the four-measure reading's 70%.

**Every one of the ten at the head is a documented departure**, each held by an `AMBIGUOUS_*` group
whose argument is the reason it is there: `issue11403_reduced.pdf` (divided consensus, read in
`CONTRADICTED_SUBSTITUTED_FONT`'s cap-height note), `bug766086.pdf` (link border), `bug1743245.pdf`
(stroke adjustment), five pages of `freeculture.pdf` (dense text at book size),
`issue4260_reduced.pdf` (zero-area fill) and `issue16224.pdf` (one ladder). **That is the result
rather than a disappointment**: the list had never been able to show them, and a corrected
instrument whose first reading is a set of notes this tree already wrote is an instrument that
agrees with the tree's own record.

### The reading that is worth more than the list

**On 31 of the 48 the closest pair sits inside all three bounds**, so the page is ambiguous on the
differing fraction alone and the ratio's denominator is a number under 1. **On 22 of the 48 our own
nearest is inside them too.** There the ratio is between two numbers that both agree with everybody
as far as the three measures can see, and it ranks a page higher the more closely the references
agree — not the further away we sit.

**The head is the sharpest instance of it.** `issue11403_reduced.pdf` page 1 is 9.06×, ours 0.51
over 0.06, and the page's own line says its disagreement is `differing alone, 6.24%/5.00%`. So the
list's top entry is a page whose disagreement is **invisible to the three measures the list is
computed in**. Both units have a blind spot and they are different ones; neither is the reading,
which is what step 1's *read it with the picture, never alone* means when it is made arithmetic.

The nine pages where we are outside a bound while the closest pair is inside are the sublist an
opening round wants: `bug766086.pdf`, `freeculture.pdf` 315, 322, 323, 329 and 333,
`issue16224.pdf`, `endchar.pdf` and `issue12337.pdf`.

### The five book pages, measured rather than cited

Five of those nine are one book, so the cluster was measured instead of being handed to
`AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`'s population argument.

**The ink says the marks are the same marks.** A ladder on page 315 at 1×, 4× and 8× — ours through
`examples/render_at`, `poppler` through `pdftoppm -cropbox`, `mupdf` through `mutool draw`, three
ladders because one cannot tell convergence from drift:

| | 1× | 4× | 8× |
|---|---|---|---|
| ours | 11.8908 | 11.9540 | 11.9855 |
| `poppler` | 11.8704 | 11.9478 | 11.9592 |
| `mupdf` | 11.9611 | 11.9979 | 11.9914 |

All three converge, ours lies **between** the other two at every rung, and at 8× the three are
within **0.032 of 255**. On all five pages our 72-dpi ink is inside the three references' own
spread, within 0.09 of 255 of every one of them. Nothing is missing and nothing is over-painted;
what the mean, the tile and the similarity see is sub-pixel phase, which is precisely what an ink
measure cannot see and what that group has said since the two-hundred-and-twenty-ninth session.

**What puts these five at the head is the denominator, and it is trap 9 in a place this tree had
not priced it.** Over the book's 321 compared pages, `poppler` and `mupdf` — the two voting
references that share `libfreetype.so.6`, where `ghostscript` carries its own statically linked copy
— are the closest pair on **9 of the 11 pages that reach this list** and on only **7 of the other
310**, and their own median MAE is **724** over those 11 against **1760** over the other 310.
Trap 9 has always been about shared code manufacturing an agreement in the **numerator** of a
verdict. Here it sharpens a **ratio's
denominator**, and the page it lifts is one every instrument agrees is fine.

## Consequences

- Step 1 has a printed, ordered queue in the unit its own calibration is in, and two counts that say
  when its ratio is not measuring what its name says.
- A round taking this list opens the nine, not the 48 and not the 569.
- The ranking is not a ratchet and decides no verdict, exactly like the three beside it.

## What this does not settle

- **A ratio whose denominator is below 1 has no floor**, and nothing bounds how large it can get.
  The counts say how much of the list is that shape; they do not order the list differently, and
  whether the ranking should require our own number to be outside a bound is a question a round with
  two readings of the same pool can now ask.
- **Neither unit contains the differing fraction and the page's own verdict**, which is why the head
  page's disagreement is unreadable from either column. `doc/todo/12`'s 278 pages are still owed and
  are the same subject seen from the bound's end.
