# 748 — The work nothing bounded

Fourteenth merge round of the block. Four branches, **no conflicts**, and a batch in which three of
four rounds found that a thing this project believed it had — a bound, a budget, a unit — was not
there.

## The sequence, whole, on a quiet machine (load 1.10)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2656 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 61 contradicted, 836 ambiguous, 3 our
geometry, 2 reference geometry, 42 not comparable, 18 no render** · `render-quorra` **933 agree, 22
differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445
implemented, 223 partial, 0 unreviewed**.

## 745 — nothing bounded the work, and there was no budget to derive

The bounds on a host drawing marks, enumerated out of the code rather than assumed:

| layer | bounds | value |
|---|---|---|
| frame header | the whole **message** | `MAX_MESSAGE`, 2 GiB |
| `display_list::count` | each count against the bytes behind it | `least::COMMAND` is 1 byte |
| `protocol::decode` | the **target**, before it is an allocation | `MAX_PIXELS`, 2²⁸ |
| `TargetSpec::for_page` | either **dimension** | `MAX_EXTENT`, 2²⁴ |
| decoder and backend | **nesting** | `MAX_GROUP_DEPTH`, 16 |
| — | **the work** | **nothing at all** |

**No rasteriser in this tree contained a cancel, interrupt, deadline or abort**, and `render-quorra`'s
budget refusals are not the exception they look like — that budget is the device's *resources*, not a
frame's cost. With measured terms (99 bytes and 2.76 ms a page-covering fill) **a message the wire
already permits is about seventeen hours of drawing**, in the unconfined process.

**And the budget could not be derived**, which contradicts the briefing's premise outright. The tree's
only pre-draw estimate — already computed on every CPU draw to place its strips — correlates with the
measured draw at **Pearson 0.115**. Two pages of the same size settle it: cover 0.2 draws for 162 ms,
cover 593 draws in 15.9 ms — the estimate 3000× apart, the clock 10× apart *the other way*. So the
answer is an **interrupt**, not a budget: raised and honoured at the top of `encode`'s command loop,
and deliberately not called a cancel, because a `Canceller` ends a process while this suffices only
*because the loop is ours*. Device backends were given no such method rather than one they would
ignore.

**+0.0003%** instructions with no interrupt present, **+0.18%** for a caller that asks to be able to
stop, byte equality asserted over seven scenes. Measured: the worker answers in 14 ms with 990 453 B
of marks it never draws, the host draws them for **27.6 s**, and an interrupt returns the thread in
**1.3 / 1.9 / 2.1 ms** — one command, exactly the granularity claimed.

Second finding, from reading rather than measuring: **`Stale::plan` stands in until a rendering
*lands*, and an interrupted draw never lands** — so a host raising one owes the viewer a failure
message or the stand-in becomes permanent. Honestly scoped: whether it fires cannot be established
from this boundary.

## 747 — a price four numbers old, and three roads nobody had priced

`doc/todo/40`'s price had never been re-run, on code six ADRs have moved since. It had decayed:

| the file said | today |
|---|---|
| 13.83 G instructions, 20 rasterisations | **17.47 G** |
| `MaskCache::get` 24.3% of that | **29.96%** (35.4% of a rasterisation, one-off split off) |

**And the item's third bullet had offered three roads with no price on any of them for three hundred
rounds.** Simulated: the *exact* arm saves **5.6%** of that page's scanned mask rows, the departure
**51.1%**; corpus-wide −4.0% against −14.0%. The exact arm is not short of candidates — half the
page's non-root nodes share their parent's band — but the sharing is one-to-one, so building an
intermediate to serve a single leaf **moves work rather than removing it**. *There is no cheap exact
version of this item*, recorded rather than argued.

What shipped came from asking the same census one further question: **10 596 of 14 253 chain steps
state a rectangle admitting every pixel of the band**, and §10.7.4 defines the region as "the set of
pixels that would be included by a fill operation", so such a step contributes the whole set and the
composing `min` is the identity — and declining it carries nothing between bands, so ADR 0219's
arithmetic never enters.

| | before | after |
|---|---|---|
| `bug1721218_reduced.pdf`, 20 rasterisations | 17 471 143 843 | **15 807 895 387 (−9.52%)** |
| per rasterisation | 739 915 947 | **657 936 980 (−11.1%)** |
| `MaskCache::get` inclusive | 5 233 622 983 | **3 570 743 625 (−31.8%)** |

**`raster_digest` byte-identical on all 957 corpus first pages.** The two text-page regressions are
attributed: 47 522 instructions of a 12.5 M delta are the cache, the rest is binary layout.

Three method notes, all worth keeping. **Trap 13 needed inverting** — a byte-identical saving cannot
be calibrated against a document, so the calibration is a *planted defect* that fails two of three new
tests. **A census that answers your question can still be measuring something else**: its first
version predicted three steps in four where the code dropped two, and the gap was a condition the
census had no reason to model — *ten minutes of `eprintln` closed what an afternoon of reasoning
would not*. And a page-covering clip stated as a rectangle thousands of pages across is one
`tiny-skia`'s fixed point cannot express, so answering from the rectangle's containment rather than
from what the converter would have done is both the recovery and the stronger reading.

## 744 — the queue was in a consistent unit, and it was the wrong one

741 fixed a line that printed two instruments as one. **The census it added counts in four measures**,
because that was the only unit the closest pair's number existed in — and that reading names **569 of
804** complete pages, seven in ten. The unit that reproduces session 518's hand-taken figure had no
number for the pair at all, so 744 built one. **The list is 48**, exactly ADR 0643's prediction, now
printed by the gate rather than computed by hand.

**All ten at the head are documented departures**, reported as the result rather than dressed up. The
finding is the ratio: **neither column has a floor at 1.** On 31 of the 48 the closest pair sits inside
all three bounds and on 22 our own nearest does too — so the ratio is between two numbers that agree
with everybody, and **it ranks a page higher the more closely the references agree.** Both units have
a blind spot and they are different ones; the gate prints both counts now.

**Trap 9 gains a tenth mechanism**, and it is a genuinely new shape. Three ladders on one book
converge with our value *between* the two references at every rung, all three inside 0.032 of 255 at
8×; what lifts those pages is the **denominator** — the two references sharing a font library are the
closest pair on 9 of the 11 pages that reach the list against 7 of the other 310. *Trap 9 is a list of
ways shared code manufactures an agreement; in a ratio that agreement is the divisor, so the same
mechanism accuses us.* And both references reproduce session 233's ladder **to the three decimals it
printed, 511 rounds later**.

## 746 — the rule's head moved by decay, and a plateau is the normal case

The successor selection rule's third use, and **the head moved for the first time by decay** — the
rows the last two uses read are off the ranking entirely, which is the property it was chosen for.
But it is **a plateau, not a peak**: three rows tie at seven annotations, and the rule ranks by count
while saying nothing about ties, which are the *normal* case because one issue lands one to seven
annotations on a row. All three were read far enough to break it, and the tie-break is now part of the
rule: **read the row whose errata strike a cell — a requirement level, a type, a description — ahead
of the row whose errata substitute a word in prose.**

**The finding is a blocker that charged two legs to the second one.** A row and a doc comment both said
turning a page coordinate into a latitude needs the EPSG registry. An erratum writes the *shape* of an
entry the published table only counts — a 4×4 affine matrix in row order — and that entry has priority,
so on such a file **the object-to-projected leg is a matrix multiply with nothing outside the standard
in it**. Its argument for implementing from an erratum rather than merely quoting it: *twelve numbers
for a 4×4 matrix is §8.3.4's own elision one dimension up.*

**Trap 13 caught a fixture that would have passed a real defect**: the first version passed the
*transposed* plant, because a diagonal matrix agrees with its own transpose.

Two things worth carrying. The instrument was **sound this time**, after two of three uses found it
broken. And the erratum's strike is three words — under `check`'s four-word floor — **for the second
consecutive use**, which says that is where the errata debt now sits. It is also the second erratum in
two uses that **vindicates code rather than correcting it**.

## What the batch has in common

| believed present | found |
|---|---|
| a bound on the host's draw | nothing at all — seventeen hours from a permitted message |
| a cost estimate a budget could be derived from | Pearson 0.115, and two pages that settle it |
| a price for the clip chain's three roads | none, for three hundred rounds |
| a queue in the calibrated unit | the unit that names 70% of the pool |

## Owed

- **A policy for the interrupt** — nothing decides *when* to raise one, because no host is on this
  boundary yet — and the failure message it owes the stale-frame machinery.
- **The other four of 744's nine** — named and not re-derived — and the question its ranking now
  raises: should it require our own number to be outside a bound?
- **`check`'s four-word floor**, where the errata debt now sits on two consecutive uses.
- **Orca on all three binaries, by a person.**
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
