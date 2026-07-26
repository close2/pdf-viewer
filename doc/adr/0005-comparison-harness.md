# ADR 0005 — Reference-comparison harness and its measured limits

Status: accepted, 2026-07-26.

## Context

"Is our rendering correct?" has no local answer. The specification is prose, the corpus
is the whole web, and the only practical oracle is what other implementations do. But no
two renderers agree pixel-for-pixel, so "differs from poppler" is not evidence of a bug.

## Decision

`tools/pdfref` compares against three independent implementations — poppler, mupdf and
ghostscript, chosen because they share no rendering code — under a **triangulation
rule**:

- **Two or more references agree with each other and we differ** → a real bug. Two
  unrelated implementations reaching the same answer is strong evidence it is right.
  Fails the build.
- **The references disagree among themselves** → an ambiguous corner of the
  specification. Recorded, but *not* a failure: there is no correct answer to hold us to.
- **Fewer than two references available** → failure. A comparison suite silently
  degrading to nothing is the failure mode this design exists to prevent.

Consensus is computed as the largest *mutually* agreeing subset, checked exhaustively.
Counting pairwise agreements would be wrong: A agreeing with B and B with C does not make
A agree with C, and treating it as if it did would let a chain of near-misses masquerade
as consensus.

## How it works before a parser exists

`test-scenes` holds the same page twice — `basic()` as a display list, `basic_pdf()` as
PDF bytes. The harness renders the PDF externally and the display list with our own
backend. Both live in one file so a reviewer can check them against each other.

That is deliberate sequencing. A harness written *after* the parser tends to get tuned
until it passes; built first, it is trusted to say when something is wrong. When
`pdf-syntax` lands, the display list will be produced *from* the PDF and this becomes a
true end-to-end check with no change to the harness.

## Measured results

On the `basic` fixture at 72 dpi our CPU backend is **byte-identical to mupdf** (mean
0.0000), differs from poppler by 0.0016 and from ghostscript by 0.0336 — in every case
*less* than the references differ from each other. All four renderings are visually
indistinguishable.

## Two findings that changed the design

**Renderers disagree about page dimensions.** A4 is 595.276 units wide; poppler and
mupdf produce 596 pixels, ghostscript 595. Every A4 document failed comparison outright
on a difference that is nobody's bug. `normalise::to_common_size` now crops to the
smallest common size when the spread is at most one pixel per axis, and reports that it
did. The bound stays tight deliberately: a two-pixel difference is not rounding, and
absorbing it would hide the `MediaBox` and `CropBox` misreadings the harness is for.

**Pixel comparison cannot police text.** On the specification PDFs the three references
disagree with each other at a worst-tile error of 26 to 28, with 2.7% of pixels
differing. A difference map shows the disagreement confined to glyph outlines and
single-pixel shape borders — filled interiors are identical — so it is hinting and
antialiasing, not error.

The consequence is that the noise floor on text pages sits above the signal. No single
tolerance can both accept that and catch a genuinely wrong glyph. So:

- `Tolerance::VECTOR` (the default) is tight — mean 1.0, worst tile 5.0 — measured
  against a floor of 0.4 to 1.1.
- `Tolerance::TEXT_HEAVY` is loose enough to accept the measured floor, and is documented
  as a **weak gate** that catches only gross failures.
- **Text correctness therefore belongs to the text-extraction metric**, comparing our
  extraction against `pdftotext`, which checks encoding and `ToUnicode` handling
  independently of how glyphs are painted. This raises that metric from "nice to have" in
  the plan to load-bearing.

## Consequences

Failure artefacts — our render, a side-by-side, and per-reference heatmaps — are written
automatically and kept, not cleaned up. On a disagreement they are the evidence, and a
suite whose failures take manual work to diagnose gets ignored.

Renderer versions are recorded in every report, because these tools change their output
between releases: a difference appearing tomorrow may be an upstream change rather than
our regression.

`pdfium` remains a worthwhile fourth reference, being Chrome's renderer and therefore the
de facto standard, but it is not in the main Arch repositories.
