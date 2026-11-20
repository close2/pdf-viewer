# 3266 — A code on `.notdef` drawn from a machine's face is held to the nothing the window drew

Session 2141, an archive round in the `gwg` worktree. Status: **accepted** and built. Context: ADRs
2458, 2517, 3125, 3126; `doc/pdfa-blocking-issues.md` R08 and R03 (`219789`);
`crates/pdf-transform/src/archive/fonts/described.rs` (`same_glyph`). Clauses: ISO 32000-2
§7.3.4.2, §9.6.5; ISO 19005-2 sections 6.2.11.4.1, 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1,
6.2.10.9.

## Context

`219789` stopped under `as-if-printed` at "a page that would be photographed is too large to be
photographed … within the pixel budget". Printed at the refusal: one font,
`TimesNewRomanPS-BoldMT` (TrueType, `/WinAnsiEncoding`, not embedded, drawn from this machine's
face), was deferred to the photograph on every one of the 54 pages, with
`DRAWN_DIFFERENTLY` from `proved`: code 10 — a line feed, which `/WinAnsiEncoding` names nothing
for — reached no glyph in the shipped face (`shipped_glyph` `None`) and reached the empty glyph
`super::empty` had given it (glyph 230, no outline) in the converted font. The shipped-face arms of
`same_glyph` accept an empty glyph where the window drew none; the machine-face arm demanded the
shipped face's own glyph, which a code on `.notdef` has none of. 54 letter pages at 300 dpi are
454 million pixels against ADR 2517's 2^28, and 54 pages against its 32.

## Decision

The machine-face arm accepts, for a code the shipped face reaches no glyph for, a converted glyph
other than `.notdef` that draws nothing, where `empty::reaches_notdef` holds of the source:
`drawn_from` already admits a machine face only where each such code draws nothing from it. This
is the rule the other arms state, applied to the arm that lacked it; the advance is held by
`same_metrics` as before.

**The pixel budget is not changed.** With the font restated, only the 19 pages the fence names are
photographed — 160 million pixels, under both of ADR 2517's bounds — and the other 35 keep their
own content. The remedy is the page's own content kept where it no longer needs the photograph,
which keeps the text a reader can extract; a lower resolution or tiled photographs would have
photographed 35 pages that need none.

## Measured, and one checker disagreement

`219789` converts at part 4 under `as-if-printed` (2b stops at section 6.1.13, which the two-target
request answers, ADR 2498). veraPDF 1.30.3 fails it at 6.2.10.7 on one glyph: page 53's damaged
stream holds a literal string spanning carriage returns, which veraPDF (and mupdf) read as code 13,
where §7.3.4.2 has "an end-of-line marker appearing within a literal string without a preceding
REVERSE SOLIDUS shall be treated as a byte value of (0Ah)"; this tree reads code 10 and gives it the
empty glyph. The standard reads for this tree; the disagreement is recorded beside section 1's
others. Test: `a_code_on_notdef_drawn_from_a_machines_face_is_given_an_empty_glyph`.
