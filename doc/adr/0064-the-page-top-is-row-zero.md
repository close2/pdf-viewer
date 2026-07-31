# ADR 0064 — The page's top edge is raster row zero

Status: accepted, 2026-07-31.

## Context

The handover's demand track says to rank `CONTRADICTED_UNEXPLAINED` by our worst measurement
over the bound it is held to, and open the artefact at the top of the list. That is
`issue3694_reduced.pdf` page 1, at **1.81** — 12.47% of pixels differing, the largest share on
the list.

Its crop box is `[0.0 735.878 272.595 792.0]`: **272.595 by 56.122 units**. Four reference
renderers put the one line of type on it at raster rows 12 to 28. We put it at 13 to 29.

## The measurement that named it

`poppler` and `mupdf` produce a 273×57 raster; `ghostscript` and we produce 273×56. That is
the signature of the `CONTRADICTED_PAGE_ROUNDING` group — a fractional page rounded two ways —
and it is what the page would have been filed under if anybody had grouped it.

**It is not a rounding difference, and `ghostscript` is what says so.** Its raster is the same
size as ours and its content is one row higher. A disagreement about how many rows a page gets
cannot move content within a raster of a given height; only a disagreement about where the page
*starts* can.

Our own render measures 273×**57**, not the 273×56 in the artefact — the harness resizes to a
common size before comparing — and its transform is `scale(1, -1)` then `translate(0, 57.0)`.

## Decision

**`TargetSpec::for_page` flips y about the page's height in device units, not about the
raster's pixel height.**

```rust
let exact_height = f64::from(list.page_size.height) * f64::from(scale);
…
transform: Transform::scale(scale, -scale)
    .then(Transform::translate(0.0, height_as_f32(exact_height))),
```

`pixel_extent` rounds *up*, so that the raster contains the whole page. A page 56.122 units
tall therefore gets 57 rows and 0.878 of a row is spare. The old code translated by 57, which
put the spare fraction at the **top** and pushed every mark on the page down by it. The new code
translates by 56.122, which puts it at the **bottom**.

Three reasons, in the order they carry weight:

1. **The x axis already decides this, and decides it the other way.** Nothing flips x, so the
   0.405 of a column spare on this page is at the right — the crop box's left edge is device
   x = 0. Anchoring y to the raster's last row means anchoring x to the left edge and y to the
   bottom edge, which is the same corner in neither convention. The crop box's top-left corner
   is the raster's origin, in both axes or in neither.
2. **The standard states nothing, so this is a documented choice, and it is not the *rounding*
   choice.** §10.7 leaves scan conversion to the device and says nothing about a page whose size
   is not a whole number of pixels. How many pixels a fractional page gets is genuinely open —
   we and `ghostscript` round, `poppler` and `mupdf` ceil, and four defensible answers exist.
   Where the page's origin lands is a different question, and all four references answer it the
   same way.
3. **It was measured.** 11 of the corpus's 76 contradicted pages agree after the change, and
   none of them disagreed for a reason anybody had written down.

## Consequences

**Contradicted pages fall 76 → 65; agreeing pages rise 821 → 832.** Eleven pages, from four
different groups:

| left | group it was in |
|---|---|
| `bug1065245.pdf`, `bug1922766.pdf`, `bug1934157.pdf`, `issue12963.pdf` p6 | page rounding |
| `bug1650302_reduced.pdf`, `freeculture.pdf` pp. 67, 76, 339, `issue1002.pdf` | unexplained |
| `bug1671312_reduced.pdf` | substituted font |
| `issue20232.pdf` | symbolic font flags |

`CONTRADICTED_PAGE_ROUNDING` is halved, from 8 to 4, and that is the finding worth keeping:
**a group whose name is true of every member can be causal for only half of them, and it will
go on explaining them after it has stopped being the explanation.** Every one of those four
pages does have a fractional page box and does round differently from two references. That was
never why they were contradicted.

`issue20232.pdf` leaving is the case to read carefully. It is on this list because a symbolic
`TrueType` subset embeds an empty outline where the drawing's `/Differences` names a diameter
sign, and **that is still true** — the drawing still reads `56` where three references read
`⌀56`. What changed is that a one-glyph difference on a 595×842 engineering drawing sits inside
the bound once the row is back where it belongs. Its entry is kept, empty, saying so: *a page
leaving a contradicted list is not the same as a page being right.*

**`issue3694_reduced.pdf` itself stays**, at 0.60 instead of 1.81 — mean 3.02 against a bound of
5.00, 8.93% of pixels differing. What is left is hairline-outlined display type at seventeen
pixels against two references that share `FreeType`, which is the noise floor this file's own
tolerance comment describes.

**Nothing in the tree failed when the fix landed**, and that is the second finding. There *was*
a test — `transform_flips_the_y_axis`, asserting in as many words that "page top belongs at
raster row zero" — and it uses A4, whose height is 842 units and 842 pixels, where the two
conventions are the same number. The parameter it left at its default was the one that mattered.
`a_fractional_page_still_starts_at_raster_row_zero` is the same assertion at 56.122, and it was
confirmed to fail against the old translation.

**No page got worse**, checked by the gate's own newly-contradicted list, which is empty.
`corpus.rs` is unchanged: this moves marks within a raster and never changes what is drawn.
