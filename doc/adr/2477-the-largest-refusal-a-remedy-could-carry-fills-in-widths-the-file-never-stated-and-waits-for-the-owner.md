# 2477 — The largest refusal a remedy could carry fills in widths the file never stated, and waits for the owner

Status: deferred. Session 1821, the archive slot of batch one hundred and twenty-two. Context: ADR
2442 (the ranking), ADR 2458 (the empty glyph, the last remedy built from it), ADR 2460 (the census
names the pack), ADR 2146 (the standard 14 by name alone), `doc/questions/A48`, `doc/questions/Q687`.

## The ranking, re-run

`the_shipped_profiles_list_what_real_producers_files_refuse_on`, in an export of HEAD, with and
without `QUORRA_CJK_PACK` (a copy of the owner's pack, its five programs held to their digests), both
shipped profiles at 2b and part 4, pdf.js and `doc/corpora`. Without the pack `as-if-printed` 2b is
793/2/160/11 and 351/6/116/16 (converted/conforming/refused/unreadable); with it 804/2/149/11 and
351/6/116/16, part 4 807/0/148/11 and 353/0/121/16: eleven documents the pack frees under one
profile, the thirteen's other two being fence rows.

Ranked by documents, over both corpora, leaving out what ADR 2442 left out (a profile's `stop`, the
fence rows, `image-smoothing`) and the press profile's twenty-nine at part 4 (ADR 2130):

| documents | reason | alone in its document |
|---|---|---|
| 10 | a font naming a face outside the 14, no descriptor, no `/Widths` | 10 |
| 8 | a code the encoding sends to `.notdef`, beside a stated descriptor | 0 |
| 7 | a DeviceN colourant with no Separation array to name it | 7 |
| 7 | a header stating no version this tree reads | 7 |
| 7 | a TrueType code reaching a glyph by a mapping the route excludes | 3 |
| 5 | an `Adobe-Identity` font (re-keying's population) | 5 |

The second row builds nothing: each of its eight documents also stops on another reason. The third and
fourth would invent a tint transform and a version. The fifth is a route's own boundary.

## The first row is not the converter's to carry

The brief's hypothesis, that the largest carryable reason holds under ten documents, held as a count
and not as a remedy. The ten name `/Arial` (three), `/Tahoma`, `/Monaco`, `/Wingdings`,
`/TimesNewRoman`, `/Times-Italic` under `/Type7`, a descriptor spelled `/GontDescriptor`, and a font
with no `/BaseFont`. None states `/Widths`. Table 109 makes `/Widths` and `/FontDescriptor` required
for every font but the 14, and Annex D publishes the 14's metrics; for the rest the standard states
no layout.

In an export, `described::describe` was opened to any `Type1`, `MMType1` or `TrueType` simple font
(its proof unchanged: the converted font read back against the viewer's own drawing, code by code,
advances to a thousandth of an em). Eight of the ten convert at 2b and 4 under `as-if-printed`;
veraPDF 1.30.3 passes fourteen of the sixteen outputs and fails the one with no `/BaseFont`
(6.2.11.2, test 3). The route is therefore feasible, and the reason it is not built is the
widths: the viewer lays an absent-width `/Arial` out by Helvetica's metrics "in practice"
(`pdf_font::standard_metrics`), which is a convention and not a clause, and writing the result into
the file states one reader's choice as the producer's. A48 names that line: "state an interpretation
the standard defines; never fill in an absence". The refusal's current sentence calls it "not built
yet", which is untrue of a decision.

## The decision

1. **Nothing is built for the first row.** `describe` is as it was; the experiment is not in the
   tree. `doc/questions/Q687` asks the owner whether to keep the refusal (recommended, reworded as
   the fence), or to lift it behind a loss of its own that `as-if-printed` discards and `as-shown`
   stops on.
2. **The fallback slot item is built instead**: ADR 2476, the descriptor Table 120 names.

## Consequences

- The ledger-free census is unchanged by this round: the counts above are HEAD's.
- If Q687 is answered (b), the work is `describe`'s guard, a loss, a refusal for a missing
  `/BaseFont`, and these eight documents' veraPDF verdicts.
