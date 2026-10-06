# 1572 — `issue4436r.pdf` is held by the references' departure, and no group is a departure of ours

Session 1368. Status: **accepted**. Amends ADR 1483 section 2's verdict for one group; moves no
row and no pixel. Context: ISO 32000-2 §10.7.4, §8.9.6.2, §8.9.7; ADRs 0499, 0805, 1082, 1483, 1512, 1560;
`crates/pdf-model/tests/oracle.rs`'s `CONTRADICTED_SUBPIXEL_IMAGE` and `WHOSE_DEPARTURE`;
`tools/pdfref`'s `Tolerance::widened_to`.

## 1. The question

`tools/state.sh oracle-held` named one page held as a departure of ours: `issue4436r.pdf` page 1,
a 1x1 image mask under `180 0 0 -0.48 10 25 cm`. ADR 1560's method was asked of it: write the
closed form from the file's own bytes, put it against today's references with
`examples/compare_rasters` and the gate's bound, and say whose departure the verdict is.

## 2. What the words say

§10.7.4's image paragraph governs. The mask is an inline image, which §8.9.7 makes "a sampled
image" specified in another form, and §8.9.6.2's mask is an image whose samples designate where
the current colour is painted: "However, only those pixels whose centres lie within the region
shall be painted." The mask's region is
device x [10, 190), y [25, 25.48) on the 200x50 page; row 25's centres sit at y 25.5, so the clause
paints nothing. Ours paints row 25 at `0x85` (0.478 of a row, §10.7.4 row's departure (1)); all
four references paint it black, which is the shape rule — "any pixel whose half-open square region
intersects the shape" — applied to an image, the paragraph before the one that governs.

## 3. Measured

The inline image's one sample is `0x20`'s first bit, 0, painted under the default `/Decode`. A
script, not committed, wrote three forms from that geometry over our own raster (re-rendered with
`examples/render_at`, byte-identical to the oracle's `issue4436r-p1-ours.png` of 2026-10-06): the
clause (row 25 white), the area (0.48 of the row, which is ours to the level) and the shape rule
(row 25 black). Against the oracle's cached references of 2026-10-06:

| raster | vs `poppler` | vs `mupdf` | ssim vs `poppler` |
|---|---|---|---|
| ours = the area | 8.06% | 7.55% | 0.948 |
| the clause | 8.06% | 7.55% | 0.821 |
| the shape rule | 6.71% | 6.20% | 0.985 |
| `poppler` vs `mupdf` | 3.67% | | 0.984 |

The bound is `Tolerance::TEXT_HEAVY` widened to twice the voting pair: differing fraction
max(5%, 7.34%) = 7.34%, structural similarity min(0.90, 0.969) = 0.90. Ours and the clause both
fail on the differing fraction at 1.10x, and by the same count, because the measure is a
threshold: a white channel and a `0x85` channel each differ from the references' black. The
clause is further out on every other measure. Only the references' own row clears the bound.

## 4. The decision

The clause's own form put in our place is contradicted by the same pair on the same measure, so
the verdict is held by the references' departure from the image paragraph, not by departure (1):
`CONTRADICTED_SUBPIXEL_IMAGE` is `Whose::References` under §10.7.4. Departure (1) is still on the
page and still a departure, priced on §10.7.4's row; no reading of the clause available to this
tree closes the verdict, and moving to the references' whole row would be curve-fitting to them.
No pixel moves.

So the held pool is 0 / 14 / 33 by `WHOSE_DEPARTURE`, and no contradicted page is held as a
departure of ours: every contradiction left is a reference's or a choice the clause leaves open.
The ranking's sentence for a fully held pool now says "if any", and its last line is the "no page
is held as a departure of ours" branch both it and `conformance::held` already had. No owner
question: the standard answers this.

The §10.7.4 row's note owes one sentence, written by the round that owns the ledger this batch:
this page is not departure (1)'s witness in the gate's verdict either, and why.
