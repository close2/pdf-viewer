# ADR 0113 — The only default bounding box the standard states

Status: accepted, 2026-08-01.

## Context

Two of the corpus's ten annotation reports discard an annotation whole over one entry:

- `checkbox-bad-appearance.pdf`: "Widget: appearance stream has no /BBox".
- `issue7446.pdf`: ": no appearance stream, and its clause states no geometry" — a report that
  begins with a colon, because the annotation states no `/Subtype` and the name interpolated
  before it was the empty string.

The second is a wording defect and is fixed by naming the absence. The first is the same shape
as ADR 0106, ADR 0111 and ADR 0112: an entry that is missing, and a refusal that throws away
everything else the annotation states.

## The argument

§8.10.2 makes `/BBox` required of a form `XObject` and §12.5.5's algorithm begins by
transforming it, so a stream without one genuinely states no box to map onto `/Rect`. There is
no reading of §12.5.5 that recovers the *scale*.

But the standard does state a default bounding box for an annotation's appearance stream, in one
place — §12.7.4.3, on the form dictionary a processor builds for a field:

> The lower-left corner of the bounding box ( BBox ) is set to coordinates (0, 0) in the form
> coordinate system. The box's top and right coordinates are taken from the dimensions of the
> annotation rectangle (the Rect entry in the widget annotation dictionary).

That sentence is written for a *constructed* appearance, and applying it to a stored one that
omits the entry is an **extension** rather than a reading. It is recorded as one. What makes it
the right extension is the alternative: refusing discards the annotation's whole appearance over
an entry the placement algorithm needs only for a scale, and the box the sentence gives is the
one every producer of such a stream is writing against anyway.

The report stays, because §8.10.2 requires the entry and a stream whose marks lie outside this
box will still draw nothing.

**The choice of default is discriminating, not cosmetic.** Taking `/Rect` itself as the box —
the other obvious candidate — makes the placement the identity and leaves the stream's marks
wherever its own coordinates put them, which for a stream drawing at `0 0 Td` is the corner of
the *page*, and `/Rect`'s clip then removes all of it. The test pins the difference: a 10×10
square at the stream's origin, a 40×40 `/Rect` at (20, 30), and the mark lands in the
rectangle's lower-left corner at its own scale — where the same fixture *with* a 10×10 `/BBox`
puts it across the whole rectangle scaled by four.

## Consequences, measured

**The corpus's one witness gains nothing visible, and saying so is the point.**
`checkbox-bad-appearance.pdf`'s appearance stream draws `(4)` in `/ZapfDingbats` — a tick — and
that font is one of the standard 14, embedded nowhere in the file. This machine has no
ZapfDingbats face for `substitute` to find, so the display list is 37 commands either side of
the change. The behaviour is therefore defended by a synthetic fixture, which is trap 8's
converse doing its job rather than a shortcut.

All four gates are unmoved: 89 incomplete, 841 agreeing and 65 contradicted, 97.9% of
`pdftotext`'s words, 1545 dates. Tests 891 → 892.

## A note on the run this session inherited

The `hayro` totals the hundred-and-twenty-fourth session could not wait for finished afterwards
and are in the handover's table: **6.04 s over 865 complete pages against `hayro`'s 41.02 s**,
median 2.16×. Our own total fell from 7.08 s while `hayro`'s barely moved (41.28 → 41.02),
which is *not* claimed as an improvement here — the interpretation A/B taken the same day puts
four sessions of change at +0.16%, and nothing in them touched a rasterising path. The fall is
the machine, and the table's standing caution is why it is printed rather than celebrated.
