# Shapes that still disappear

Status: **measured, not fixed.**
Priority: 11
Corpus: 4 known witnesses; all three shapes are general
Clauses: §10.7.4 — see `_scan-conversion.md`
Code: `crates/render-cpu/src/lib.rs`, `crates/pdf-model/src/content.rs`'s `tile`

Two leftovers from the hundred-and-eighty-sixth to -eighth sessions, which closed §10.7.4's
"no shape ever disappears" for a fill with *no* area (ADR 0154) and for a redundant pattern-cell
clip (ADR 0155). Both of these are the same sentence one step along, and neither is the
anti-aliasing departure.

## 1. A fill under an eighth of a device pixel thick

`tiny-skia` samples four times per row and rounds, so a filled sliver **with** an area vanishes
on the CPU backend. Measured on an 80-unit rule at scale 1.0, ink against the area's own answer:

```text
0.05 units → 0      0.1 → 0      0.2 → 19.8 of 16      0.5 → 39.8 of 40
```

(`render-cpu/tests/zero_area_fill.rs` records the ladder in its comment.) So a shape does
disappear, and it disappears because of the *device's* coverage quantum rather than because of
its geometry — which is why it is a rule of its own and not an extension of ADR 0154's.

**Unmeasured**: the GPU backends' own quantum. `render-quorra/tests/corpus.rs` at several scales
would show it if it differs.

**Not obvious what the fix is.** Promoting every sub-quantum fill to a full mark would fight the
anti-aliasing departure on ordinary thin shapes; the honest statement of what this tree wants is
"coverage proportional to area, but never rounded to nothing where the shape intersects the
pixel", and that is a rasteriser-level rule rather than a display-list one.

## 2. A tiling cell's two halves, composited rather than added

`issue16038.pdf`'s second square: the pattern's rule sits **on** the cell's `/BBox` edge and is
*meant* to be halved, so each half is drawn by a different cell and the two composite as
`1 − (1−a)(1−b)` rather than adding. Interior coverage 0.1159 against the geometry's 0.1333 —
13% short, where the first square is now within 0.8% (ADR 0155).

Removing that clip is not the answer: it would draw the rule twice at full width, which is what
`mupdf` does and what makes its two squares differ by a factor of 1.63 where they should be
equal. **The fix is rasterising a tiling's coverage once rather than cell by cell**, which is a
different construction from anything in the tree today — the cells would have to accumulate into
one coverage buffer before compositing. §8.7.3.1's NOTE 2 recommends treating all tiles as a
single transparency group for a related reason ("artifacts due to multiple marking of pixels
along the boundaries between adjacent tiles"), and `tile` already builds that group where the
state composites non-trivially; the group does not fix this, because the loss is *inside* it.

## 3. A sub-pixel *stroke* within half a pixel of the raster's edge

Found in the two-hundred-and-fifteenth session, diagnosing `vertical.pdf` pages 2 and 3 off
§3a's ranking. The whole content of each is two rules a tenth of a user unit wide, one 0.05
below the page's top edge and one 0.05 above its bottom.

A synthetic page with the same box and five identical rules — at the top edge, at y 300, 160, 20
and at the bottom edge — measured at scale 1, as a fraction of full coverage across the row:

```text
top edge  0.055   y 300  0.047 + 0.051   y 160  0.047 + 0.051
                  y 20   0.047 + 0.051   bottom edge  0.051 + 0.047
```

Four of the five carry **0.098 of an expected 0.1**. The one whose outer edge lies on the page's
*top* carries 0.055 — a little over half.

**Why**, and it is the same class as item 1 rather than a new one: `tiny-skia` draws a stroke
under a pixel wide as a hairline **smeared symmetrically about the path** rather than as an exact
area, which the ladder shows on its own — every interior rule splits 0.047/0.051 across two rows
*whatever its sub-pixel position*, where the exact answer for the rule at y 300 is 0.03/0.07. The
total is conserved and the placement is approximate, so a rule whose centre is half a pixel from
row zero loses the half of its smear that falls above the raster.

**Only the top and left edges lose**, because `TargetSpec::for_page` rounds the raster *up* to
contain the page (ADR 0064): the spare fraction of a row is at the bottom and the spare fraction
of a column at the right, so a mark at those edges has somewhere for its smear to go.

**Unmeasured**: whether the GPU backends do the same. The comparison exists —
`render-quorra/tests/corpus.rs` hands both backends the same display list — but it only ever
draws page one, and both witnesses are pages 2 and 3.

**What the fix would be is the same open question as item 1's.** The honest statement is
"coverage proportional to area", and getting it means not using the rasteriser's hairline path
for a stroke this thin — either by converting a sub-pixel stroke to the fill of its own outline,
which lands it straight into item 1's quantum, or by asking the rasteriser for something it does
not offer. Neither is a display-list change, which is why all three items sit in one file.
