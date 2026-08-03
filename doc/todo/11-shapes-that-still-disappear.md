# Shapes that still disappear

Status: **measured, not fixed.**
Priority: 11
Corpus: 2 known witnesses; both shapes are general
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
