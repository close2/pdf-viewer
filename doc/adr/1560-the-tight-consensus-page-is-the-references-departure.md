# 1560 — `issue7891_bc1.pdf` is held by the references' departure, not by departure (1)

Session 1362. Status: **accepted**. Amends ADR 1483 section 2's verdict for one group; moves no
row and no pixel. Context: ISO 32000-2 §10.7.1, §10.7.4, §10.7.5, §8.9.5.1, §11.3.7.2, §11.5.1;
ADRs 0025, 0489, 0492, 0805, 1483, 1512; `crates/pdf-model/tests/oracle.rs`'s
`CONTRADICTED_TIGHT_CONSENSUS` and `WHOSE_DEPARTURE`.

## 1. The question

`tools/state.sh oracle-held` named two pages as departures of ours, and `issue7891_bc1.pdf` page 1
was the second: ADR 1483 held it as §10.7.4 row's departure (1), a clip's partly covered row painted
partly, on the clipping paragraph. The contract asked three things: does the departure still stand
on the standard's words, would a correct reading close the gap, or is it a choice the standard
leaves open.

## 2. What the words say

The titles around the subject were read first: §10.7.1 to §10.7.5, §8.9.5.1, §11.3.7.2, §11.5.1.
§10.7.4's rules are normative and describe whole pixels: a shape paints "any pixel whose half-open
square region intersects the shape", and a clip is "the set of pixels that would be included by a
fill operation". No normative sentence anywhere in clause 10 or 11 allows a fractional pixel. The
two passages that expect one are informative: §10.7.1's NOTE, "The specifics of the scan
conversion algorithm are not defined as part of PDF", and §11.3.7.2's NOTE 1, "when such objects
are rasterized to device pixels, the shape values along the boundaries can be anti-aliased". A NOTE
does not grant a permission, so **departure (1) stands as a departure**, deliberate and in writing,
and the row's `departed` status is the honest one. It is not a choice the clause leaves open.

## 3. What the page fails on

The verdict is the worst tile: ours against `mupdf` 6.73 at device (224, 320), bound 6.04, which is
twice `mupdf` against `ghostscript` (3.02) on the same tile. That tile is the word inside the mask,
wholly inside the mask group's `/BBox` (device x 198.8 to 362.16, y 290.536 to 357.496) and the
black fill, so no clip edge falls in it. A script, not committed, wrote the page's closed form from
the file's own bytes (object 14's 676x436 samples, object 15's matrix, Table 142's `/BC`), and
`examples/compare_rasters` put each form against the oracle's cached references of 2026-10-05:

| raster | vs `mupdf` | vs `ghostscript` |
|---|---|---|
| ours | 6.73 at (224, 320) | 5.72 at (352, 288) |
| ours, rows 290 and 357 and column 362 as the clause's set | 6.73 at (224, 320) | 6.23 at (224, 352) |
| the clause throughout the `/BBox` | 6.80 at (224, 320) | 6.23 at (224, 352) |

On the deciding tile alone: ours is 0.16 from the box average and 0.95 from the clause's point form;
`mupdf` is 6.80 from the point form, `ghostscript` 4.68, `poppler` 4.36, `hayro` 3.01. These match
ADR 0489's table to the digit, so the references' rasters have not moved under it.

**So no correct reading of ours closes the gap.** Drawing the clip's edges as sets leaves the
deciding tile at 6.73. Drawing the whole mask as §10.7.4 says, which also undoes departure (3), puts
the clause in our place, and the clause is contradicted at 6.80. The margin is the voting pair's
distance from "[t]here shall not be averaging over the pixel area", and ours from that sentence is
0.95 of a level.

## 4. The decision

`CONTRADICTED_TIGHT_CONSENSUS` is `Whose::References` under §10.7.4, and `Whose::References`'s
doc now covers a tree that departs from the sentence by less than the margin, so the clause's own
form is contradicted too. Departure (1) is still on the page, and ADR 0489's split puts most of
the mean there, but none of the worst tile. The one page held as a departure of ours is now
`issue4436r.pdf`, where the departure *is* the margin (ADR 0805). No owner question: the standard
answers this.

The §10.7.4 row's note owes one sentence, written by the round that owns the ledger this batch:
this page is not departure (1)'s witness in the gate's verdict, and why.
