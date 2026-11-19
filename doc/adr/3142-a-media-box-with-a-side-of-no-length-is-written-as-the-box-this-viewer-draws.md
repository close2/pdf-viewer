# 3142 — A media box with a side of no length is written as the box this viewer draws

Session 2110, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R03 row
("left: a MediaBox with a side of no length"). Status: accepted and built. In `pdf-transform`:
`archive/larger_units.rs` (`restate`, `as_drawn`, `has_area`, `stated_user_unit`; `Restatement`'s
`user_unit` is now optional), `archive/boundaries.rs` (`THE_MEDIA_BOX_IS_THE_PAGE`'s sentence),
`archive/rewrite.rs` (`PageBoundaryRemoved`'s sentence). Test:
`crates/pdf-transform/tests/archive/finer_remedies.rs`
`a_media_box_with_no_width_is_written_as_the_box_this_viewer_draws`. Context: ADR 2888 (a media box
restated in larger units), ADR 0389 (`Page::DEFAULT_MEDIA_BOX`, a choice), ADR 2440, ADR 2981 (a box
with no area read as absent), ADR 1210 (a removed boundary).
Clauses: ISO 19005-2 section 6.1.13; ISO 32000-2 §7.7.3.3 Table 31, §7.7.3.4, §7.9.5, §14.11.2.1.

## Context

`boundingBox_invalid`'s first page states `/MediaBox [0 0 0 0]` and `/CropBox [0 0 0 0]`. ISO
19005-2 section 6.1.13 holds every page boundary to at least 3 units, and ADR 2888's remedy, a
power of two under `/UserUnit`, finds no factor for a side of no length, so the document was
refused at 2b under every profile (at 4, which states no such limit, it converts).

§7.9.5 admits such a rectangle, and §7.7.3.3's Table 31 requires `/MediaBox` and gives it no
default. `pdf_model::Page` reads a box with no area as no box (`MediaBoxSubstitution::Empty`), so
the page is drawn on an ancestor's media box where one is stated, and otherwise on
`Page::DEFAULT_MEDIA_BOX` — A4, this reader's choice (ADR 0389), said in the interpreter's report.
A crop box with no area is likewise read as absent, so §14.11.2.1's default, the media box, stands.

## Decision

1. **The page states the box it is drawn on.** Where the media box a page states or inherits has a
   side of no length, the restatement writes on the page the media box `pdf_model::Page` computed,
   and each other box stating no area as the box this reader computed for it — §14.11.2.1's
   default. A box with an area is written as it was. Nothing a reader of this viewer was shown
   moves, so the decision stays `Mechanical(PageBoundaryRemoved)`.
2. **The unit and the content are the page's own.** The factor is 1: no `/UserUnit` is written,
   no bracket is added, and ADR 2888's refusals for content counted in the page's units
   (annotations, patterns, beads, destinations, layout boxes) do not apply.
3. **Reported as a restated page at factor 1**, with every box before and after, so the report
   shows `[0 0 0 0]` becoming the A4 box; the rewrite's sentence names the case.
4. **Where the box drawn is itself outside the range** (an ancestor's box past 14 400 units, say),
   the page keeps `THE_MEDIA_BOX_IS_THE_PAGE`, whose sentence now says so.

## Why this is not an invention

The A4 box is this reader's choice where the standard states none, and the output now states it
as the producer's would be stated. It invents no mark, and a conforming reader of the output shows
what this viewer showed of the source. The choice was already made by the window; writing it down
is what makes the archive show the page the person archiving it saw.

## Evidence

veraPDF 1.30.3 passes `boundingBox_invalid` written at 2b under `as-if-printed` and `as-shown`.
Drawn by this reader, the source's three pages and the output's are identical pixel for pixel.
