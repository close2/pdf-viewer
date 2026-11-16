# 2888 — A media box past part 2's range is restated in larger units

Session 2036, an archive round in the audit worktree, on `doc/pdfa-blocking-issues.md` R03's first
finer remedy. Status: accepted and built. In `pdf-transform`: `archive/larger_units.rs` (new),
`boundaries::removal`, `preserve::bracket_depths`, the rewriter's `restate_in_larger_units` (after the
photograph), `Conversion::restated_pages` (`pages_restated_in_larger_units` in `--report=json`).
Tests: `crates/pdf-transform/tests/archive/finer_remedies.rs` (three), `larger_units::tests`.
Context: ADR 1123 (the bracket around a producer's streams), ADR 1210 (the optional boxes removed),
ADR 0038 (how big a user-space unit is), ADR 0816 (the fence).
Clauses: ISO 19005-2 section 6.1.13; ISO 32000-2 §7.7.3.3 (Table 31's `/UserUnit`), §8.3.2.3,
§8.3.4, §8.4.2, §8.7.2, §14.11.2.

## Decision

ISO 19005-2 section 6.1.13 holds every page boundary to between 3 and 14 400 *units*, and Table 31's
`/UserUnit` is "a positive number that shall give the size of default user space units, in multiples
of 1/72 inch". A page that fails at its media box is restated: every box it states or inherits is
divided by `k`, `/UserUnit` is multiplied by `k`, and its `/Contents` becomes `[q-and-cm, the
producer's streams, closing]`, the first stream `q 1/k 0 0 1/k 0 0 cm`, the last one `Q` for every
state the producer left open and one for that `q` (§8.4.2's balance across the array, ADR 1123's
construction). `k` is the smallest power of two that brings every side inside the range — upward for a
page too large, downward for one too small — so each coordinate is divided exactly.

The page is the same sheet with the same marks in the same places measured in inches, and no byte of
the producer's changes; nothing is added to what a reader sees, so the provenance test of the
authoring exclusion places it on the near side, and no loss is asked of the caller: the row stays
`Mechanical` where every removal beside it costs nothing (ADR 1210). The refusal text
`THE_MEDIA_BOX_IS_THE_PAGE` called rescaling a move of every mark; under `/UserUnit` it moves none, and
the sentence now covers only the boxes no factor fits.

## What the bracket does not reach, refused by name

Only the content stream is inside the `cm`. Annotations (`/Rect`, §12.5.2), a pattern in the page's
resources (§8.7.2 maps its matrix to "the default (initial) coordinate space of the page"), article
beads and viewports, an explicit destination naming the page with a coordinate (Table 149), and any
layout attribute's `/BBox` (§14.8.5.4.3) all count the old units and are refused rather than
rewritten. So is a box with a side of no length (`boundingBox_invalid`'s `[0 0 0 0]`), content that
does not decode, a producer that pops below zero, and a nesting that the added `q` would take past 28.
A photographed page is bracketed too, its stream closing the one `q` alone.

## Evidence

`issue19517` (12 608 × 16 806) converts at 2b, 2u and 4 with `/UserUnit 2`; veraPDF 1.30.3 calls each
output compliant, and `quorra-transform render --dpi 4` of the source and the 2b output are
byte-identical PNGs.
