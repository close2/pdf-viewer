# ADR 0726 — The tail the selection gate had not read, and the frame a word is judged in

Status: accepted, 2026-08-28. Session 791, a robustness round on the text-extraction tail:
the 22 documents outside ADR 0421's selection-geometry ratchet, read as a population for the
first time since the ratchet was set.

## Context

`the_word_boxes_we_place_agree_with_the_references` (ADR 0323 instrument 1, ratcheted by ADR
0421) held at 98.26% of matched words in bounds and 486 of 508 documents fully in bounds, with
22 documents on `SELECTION_BELOW_FLOOR`. The oracle's contradicted pool carries a note per
group naming each page's mechanism and the bound it fails (`--bin unpriced` enforces it); this
ratchet's population had three loose classes in one sentence and no per-document reading. The
gate also printed less than it knew: a document's line named its fraction and worst horizontal
delta, not which of the verdict's two bounds its words fail — the first question every
diagnosis asks.

## Decision 1: the gate prints the classification it already computes

Each out-of-bounds document's line now says how its words divide between the two bounds —
`N past the horizontal bound alone, M past the vertical centre alone, K past both`, with the
worst centre beside the worst horizontal delta — and a summary line classifies the tail as a
population. `PDFVIEWER_SELECTION_DETAIL=1` prints every out-of-bounds word with both deltas,
which is the diagnosis view and is off by default.

Calibrated before being believed (trap 13): the baseline summary printed `5 fail only the
vertical centre, 17 only the horizontal edges, 0 both bounds`, the by-hand count over the same
listing's per-document lines is 5/17/0, and the per-document out-of-bounds words sum to 194 =
11163 − 10969, the verdict line's own arithmetic.

## Decision 2: a pair is judged in the word's own reading frame

The verdict's asymmetry — edges tight at 0.5 pt because they are §9.4.4's positioning
arithmetic, extents unjudged because a box's height is each extractor's ascent/descent
convention (ADR 0323 Finding 3) — is a statement about the text's **reading axis and its cross
axis**, not about the page's x and y. On a `/Rotate 90` page and under §9.7.5.1's vertical
writing mode the two swap, and the gate was holding the excluded convention to the tight bound:
`hello_world_rotated.pdf`'s two words failed at 7.45–10.15 pt of pure ascent/descent
difference while their reading-axis placement agreed to a hundredth of a point. §9.4.4 states
the displacement for both axes alike ("denoted by t x in horizontal writing mode or t y in
vertical writing mode"), so the tight bound belongs on whichever axis the text advances along.

A pair the interpreter states is vertical (`WordBox::vertical`, the axis our word's glyph
quads advance along) — and whose boxes on both sides assent by being taller than wide — is
measured on transposed boxes. The derivation
(`the_selection_bounds_against_the_references_own_spread`) is unchanged: its population is
reference against reference, where no side is the interpreter.

**Both halves of that condition were forced by watching the simpler rules fail**, which is
trap 13 run against the live defect rather than a planted one:

- the reference's box alone (the census's convention for *refusing* a drag, ADR 0421)
  transposed four documents of ordinary horizontal text — `issue6387.pdf`, `issue10640.pdf`,
  `issue12963.pdf`, `issue13447.pdf` — whose reference boxes are taller than wide only because
  their stated or obeyed metrics balloon the height (`issue4665.pdf`'s `/Ascent 3117` makes
  41 pt boxes on 15 pt words). The verdict fell to 483 of 508.
- both boxes' shapes without the interpreter transposed narrow *horizontal* words
  (`issue12963.pdf`'s `111`, `issue13447.pdf`'s `it,`), putting the ascent/descent convention
  under the tight bound with the axes' names exchanged — the very failure being removed,
  sign reversed. The verdict fell to 485.

With the interpreter's orientation in the condition: **10971/11163 words in bounds (98.28%
against 98.26%), 487 of 508 documents (against 486), no document newly out of bounds,**
`hello_world_rotated.pdf` fully in bounds and deleted from the ratchet list. The pairs total
is identical, so nothing entered or left the judged set.

A box's height is the convention quantity this instrument distrusts; the rule the two failures
teach is that **no box's shape may decide the frame a box is judged in** — only the placement,
which the interpreter knows and states.

## Decision 3: the tail is classified by mechanism, each entry priced by its failing bound

`SELECTION_BELOW_FLOOR`'s note carries the population reading: seven mechanisms over the 21
documents, each named with the bound its words fail in the gate's own vocabulary. In brief:

- **§12.7.4.3's layout hand-off** (6 documents, horizontal, exactly 1.00/2.00 pt): all
  `/NeedAppearances true` text fields. The clause hands the position to the processor
  ("positioning values it determines to be appropriate … and any layout rules it employs");
  `pdftotext`'s rule, measured from the `/Rect`, is the `/BS` width plus 2 pt; ours is
  §12.5.4's border width, so the text clears the border and nothing else. A difference of
  layout rules under an explicit hand-off is not a defect on either side, and moving to
  poppler's inset would be adopting a convention to match a reference — principle 5's
  forbidden direction. The choice stays, documented as one.
- **Table 120's pair obeyed against ADR 0216's refusal** (3, vertical centre, dx 0.00):
  `pdftotext` obeys `/Ascent 8 /Descent -2` literally (a 0.2 pt tall word box); this tree's
  plausibility band answers §9.2.2's em box. The centre measure inherits the extent
  disagreement when one box collapses — Finding 3's excluded convention arriving through the
  judged measure's denominator.
- **No stated pair** (2, vertical centre): em box against the reference's font-derived box.
- **Vertical writing judged on its reading axis** (2): `vertical.pdf` isolates a box
  convention of ours — each glyph's quad spans the horizontal ascent/descent band along the
  reading axis, where §9.7.4.3's vertical displacement is the honest per-glyph extent. Owed,
  and named in the note.
- **Substituted metrics** (5, horizontal, growing along the word): files naming no usable
  program (`/Monaco` with no `/Widths`, no `/BaseFont` at all, an embedded program with no
  `cmap`), plus the rotated `issue14497.pdf`.
- **Text at an angle the frame cannot follow** (2): oblique and sheared text put the extent
  convention on both axes, and a 90-degree transposition cannot name a diagonal reading axis.
- **`issue6127.pdf`, undiagnosed and ours** (1): two words 3.02 pt — one space advance —
  from where `pdftotext` (165.12) **and** `mutool` (165.117) both put them. Two independent
  readers agreeing against this tree is a question about our §9.4.4 arithmetic on that line
  (four fonts with `Tc` kerning between them), and the note says so rather than wearing a
  convention it has not earned.

## What was not done

- No verdict bound moved; `HORIZONTAL_BOUND`, `VERTICAL_CENTRE_BOUND` and their derivation are
  untouched.
- The widget inset was not changed to match poppler's; §12.7.4.3 hands the position over, and
  agreement with a reference is never the definition of correct.
- The vertical-mode glyph-quad extent (`vertical.pdf`'s 9.21 pt) is named as owed, not fixed
  here: it is a change to `Interpreter::show`'s quad construction with its own blast radius
  over the selection census, and this round's diff is deliberately the instrument and its
  records.
