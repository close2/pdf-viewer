# 3474 — 020747 stops at the dash bound, over a path its damaged bytes made

Session 2193. Status: **accepted**. Context: ADR 3334 (`MAX_DASHES`), ADR 2759 (a page not drawn
whole is not photographed), `doc/pdfa-blocking-issues.md` R03 and R08, principle 3.

## The question

`format-corpus/govdocs1-error-pdfs/error_set_2/020747.pdf` is refused under `as-if-printed` at
`fonts/font-programs-embedded` by "a bound this reader sets stopped its drawing" (R03, R08), and
under `as-shown` by ADR 0816's fence first. Which bound, at what figure, and is it the bound that is
wrong for this page or the page that is past any reader?

## The measurement

The bound is `pdf_render::MAX_DASHES`, 500 000 dashes a stroked path (`dash_bound.rs`, ADR 3334),
reported once, on page 7. Page 7's content stream (object 20) decodes whole and fails RFC 1950's
Adler-32 (`Damage::CheckValue`): its bytes are not the ones that were compressed. They read as the
govdocs1 error set's substitution — a period written as a digit, so `5.638` reads `51638` — and the
coordinates the page states run to `018517833438` against a 592 by 807 media box. In the content's
own space, four dashed strokes state 1.1 to 2.1 × 10⁸ dashes each over paths 0.6 to 1.2 × 10⁹ units
long, 200 to 430 times the bound; the fifth longest is under it. About two thirds of the length of
the longest lies on segments that cross the page.

## Decision

**The page's, not the bound's.** The paths that reach the bound are made of bytes the stream's own
check value disowns; no reader can recover the producer's coordinates from them, and a dasher asked
for 2 × 10⁸ dashes is what principle 3's budgets exist to stop (`tiny-skia`'s own refuses past
10⁶). The refusal stays by name: ADR 2759 does not photograph a page the window did not draw whole,
because the photograph would fix the coverage-drawn strokes into the file as the page.

**A finding about the bound, recorded and not built here.** `dash_count` measures the whole path,
including length no pixel of the page shows. A count over the path clipped to the page would let
the on-page dashes be cut as stated wherever the clipped count is within the bound — for page 7's
strokes it is far smaller — but the dasher must then be handed the clipped path at the phase its
entry point has, in all three backends (trap 2). That is `pdf-render`'s change, and the page would
still be drawn from damaged bytes.
