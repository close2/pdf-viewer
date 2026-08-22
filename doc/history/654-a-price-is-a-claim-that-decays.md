# 654 — A price is a claim that decays

Eighth merge round, four branches, **no conflicts at all** — the first clean four-way merge of this
run, and the batch that establishes a rule the project did not have.

## What was merged

`round-650`, `round-651`, `round-652`, `round-653`, branched from `0b20eb19`. Six files were touched
by more than one branch and none collided.

## The sequence, whole, on a quiet machine

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, and `cargo check --manifest-path
fuzz/Cargo.toml --bins` all silent · `nextest` **2390 passed, 17 skipped** · doctests, conformance
(163 + 5 + 1) · corpus **974 documents, 68 incomplete** · oracle **1794 pages — 908 agrees, 65
contradicted, 786 ambiguous** · `render-quorra` **957 pages — 933 agree, 22 differ, 2 refused** ·
**`fixed_documents` 40 checked, 0 absent** · text extraction, both censuses, dates, XMP, JPEG 2000 ·
`cargo deny check` all four ok. Ledger unchanged at 875 rows, no `silent` row. `--bin overstated`
prints 8, the same eight three rounds have now seen.

## The rule this batch earned: a price is a claim, and it decays

`CLAUDE.md` and `doc/habits.md` already say a ledger row's *reason* decays, that a claim about the
standard's silence decays, and that a note's *count* decays. **A price does too** — the estimate a
round writes when it measures a piece of work and declines it — and this batch found two, in
opposite directions, in one week:

- **646**: ADR 0474 priced the quarter-pixel edge fix at *nine `scan::fill` calls where there is one*
  and left the item open for that reason. **It was one call**, because `tiny-skia` already contained
  the nine pieces — and the built form is **faster** (−0.43%, −9.64%, −7.99%).
- **650**: ADR 0469 priced a shading's transfer function as `Shading::with_alpha`'s walk redone in
  `pdf-render`. **That design is wrong**, because ADR 0068's simplifier has already dropped the
  curve: a `/FunctionType 2` with `/N 1` arrives as *two stops*, so mapping them draws the chord
  between the transferred ends — **64 levels of 255** at the midpoint under a squaring transfer.

Both prices were written by rounds that had measured carefully and were right about everything else.
What moved underneath them was a *third* thing neither was looking at — a library's existing
scan converter, and a simplifier three hundred sessions older. So the rule belongs beside the others
in `doc/habits.md`:

> **A price is a claim about a tree that keeps changing.** A round that takes an item priced by an
> earlier round re-derives the price before believing it, the same way it re-derives a count or a
> population — and the cheapest re-derivation is usually asking what the libraries and the layers
> *already* contain, because that is the part a pricing round tends not to enumerate.

## The other three, in what they establish

**651 declined to move a tally, and was right to.** It examined `CONTRADICTED_NEGATIVE_LINE_WIDTH`
and found the group's *name* held for the first time in thirteen examinations — the page really is
about a negative line width — while everything under the name was wrong. So the traps file now says
**the note is the thing to distrust and the name is only its first sentence**, rather than
incrementing a count that would have been misleading.

Its measurement is the batch's best piece of instrument work. Ink ÷ length gives the width each
renderer actually painted on `issue19633.pdf`'s single diagonal rule at `-0.1 w`: ours 1.006,
`hayro` 0.996, `ghostscript` 0.564, `poppler` 0.247, `mupdf` 0.219 — **nobody painted 0.0854**.
Swept through zero at nine angles: `poppler` and `ghostscript` stroke the **magnitude**; `mupdf`
gives **two different answers** (nothing within 5° of an axis, its own 0.2-pixel floor beyond 10°).
**So the consensus that outvoted us is one renderer reading the magnitude and another showing a
floor, meeting by accident** — 1.02 and 0.00 apart at `-1 w`, and no pair at all had the rule been
horizontal. That is a fourth mechanism for trap 9, and it is not shared code, shared data or a
shared default: it is two unrelated wrong answers coinciding at one angle.

And the clause was in the wrong row. §8.4.3.2 gives the range and stops; **§8.4.1 decides it** —
values "shall be clipped into valid range", a device adjustment "shall not be stored back into the
graphics state" — a sentence already quoted in the same crate for the *miter limit*, which §8.4.1's
own list names one parameter after the line width.

**652 declined to build a sweep, with numbers.** The mirror of 645's — a parent *denying* what a
descendant asserts — is nearly free, so it measured first: **14 denied term-mentions against 125
asserted**, 3 contradicted and all noise. The asymmetry is structural — *an assertion enumerates, a
denial generalises* — and the clincher is that the mirror **would not have printed this round's own
§9.8**, because "the dimensional metrics" is neither a `/Key` nor a `Table NNN`. It also caught a
sweep catching *its own writing*: a first draft of a correction took `overstated` from 8 to 12
because "What nothing reads as an input is…" scored as an assertion, and it rewrote in the ledger's
idiom rather than widening the shared vocabulary — which is the right fix, since loosening a matcher
to admit one sentence is how a sweep stops discriminating.

**653 found the refusal was three backends' and none of them the clause's.** §8.3.4's third NOTE —
*"Use of a noninvertible matrix when painting graphics objects can result in unpredictable
behaviour"* — had never been cited by any code in this tree. `render-cpu` and `render-gpu` refused
the **whole raster** for one unpositionable mark; so did `render-quorra`, **which needs no inverse at
all**, and that is the evidence the requirement was `tiny-skia`'s and Vello's rather than the
standard's. 102 fills and strokes over 5 crawled documents were costing whole pages — 6972 commands
between them — and all five now draw. Zero in `doc/pdf.js`, so no gated page could ever have shown it.

Its by-catch is worth keeping: **`4605705.pdf`'s matrix is of full rank** (`a·d` and `b·c` agree to
every bit an `f32` has), so "singular" there is a property of the arithmetic and not of the file.

## Owed

- **`doc/todo/13`'s two**: §11.7.5.2's per-region model, and a shading pattern's colours resolved at
  the `scn` rather than at the mark — which 650 says should be taken together with §8.6.5.9's black
  point and §11.4.7's compositing target, since the `scn` resolves all three.
- **`doc/todo/11` item 8's remainder**: §10.7.4's mark for a shape its *transform* collapsed, with no
  witness anywhere and a warrant weaker than `split_collapsed_fill`'s.
- **A table in an accepted ADR moved and no session claimed it**: 651 found ours reads 0.0079 at
  0.001 device pixels where ADR 0419's table says 0. Direction argued for, cause not chased.
- **The owner's session**: `tmp/pi.pdf`, for 628. **And a push**: everything since the fuzz repair
  has never faced a CI run.
