# Q865 — May the archive restate a Type 3 glyph procedure's `d0`/`d1` width from the font's `/Widths`?

Source: round 2038 (an archive round), on `doc/pdfa-blocking-issues.md` R03 and ADR 2847 section 3. Nothing waits on
the answer: the three documents convert today under `as-if-printed` with their pages photographed, and `as-shown`
refuses them by ADR 0816's fence. No answer of yours opens the fence for this row: A694 item 3 is R11's and R03a's.

## What was found

- **The clauses.** ISO 19005-4 section 6.2.10.5 requires a Type 3 glyph procedure's width (`d0`/`d1`'s `wx`) to agree
  with the font's `/Widths`. ISO 32000-2 §9.2.4 and §9.6.4 make `/Widths` what advances the text position; the
  procedure's own `wx` moves nothing this reader draws. So writing the `/Widths` value into the operand changes no
  mark — the shape of ADR 2638's `content-as-drawn` — but it edits a glyph procedure, a content stream behind ADR 0816's
  fence.
- **The census's documents**, converted to PDF/A-4 under `as-if-printed` by the converter at main (reports in
  `~/scratch/agent/r2038/type3/`):

  | document | pages | photographed | for the Type 3 row alone | mismatches | text the photograph takes |
  |---|---|---|---|---|---|
  | `issue19634` | 1 | 1 | 1 | 486 (six fonts, glyph `g0`, `/Widths` 0, `wx` 1536) | 50 characters |
  | `issue19971` | 6 | 3 | 3 | 845 (eight fonts, glyph `g0`, `/Widths` 0, `wx` 500 to 2550) | about 2 300 characters |
  | `303226` | 19 | 19 | 0 (every page is photographed for its font `MIonic` as well) | 1 (`emlowln`, `/Widths` 7900, `wx` 1000) | none the remedy would keep |

  Every mismatch in the two pdf.js files is one shape: codes mapped by `/Differences` to `g0`, whose `/Widths` entry
  is 0 while its procedure states a positive width.

## The question

May a conversion write each such procedure's `wx` (and nothing else) as the font's `/Widths` value, under a named
loss (say `type3-width-as-drawn`), the procedure's original operand kept in `xmpMM:History`?

- **(a)** Yes, under both shipped profiles: no mark moves, the text stays text, `as-shown` converts them too.
- **(b)** Yes, under `as-if-printed` only; `as-shown` keeps the fence.
- **(c)** No: the photograph stays the answer.

## Recommendation

**(a)**. The edit is ADR 2638's construction (a number the reader already draws by, written where the standard asks
for it), and it gives back about 2 350 characters of selectable text across four pages of two documents that today
leave as images. `303226` gains nothing either way. A procedure a glyph is drawn from more than once with different
widths, or a font an annotation reaches, would keep the photograph.
