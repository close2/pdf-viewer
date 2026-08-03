# ADR 0169 — A stencil through a tiling pattern, and the tile that was three steps out

Status: accepted, 2026-08-03. Session 218. `doc/todo/20-stencil-with-a-tiling-pattern.md`, now
deleted.

## What was owed

§8.9.6.2's stencil mask "designate[s] places on the page that should either be marked with the
current colour or masked out", and §8.7.2 makes a pattern a current colour like any other. An
image sample cannot carry a pattern, so ADR 0151 separated the two halves: the stencil becomes a
§11.5.2 *alpha* soft mask and the pattern paints the image's unit square through it. That closed
the **shading** case. A **tiling** pattern is not a paint at all — it is a content stream replayed
per cell — so it was refused by name, on two corpus documents.

## The fix is one line of plumbing, because the mask was already in the right place

`Interpreter::tile` ends by putting the *state's* soft mask on the group it builds out of the
cells, and it does that because §11.6.7 asks for the cells to composite once:

> In a raster-based implementation of tiling, it is advisable to treat all tiles as a single
> transparency group. This avoids artifacts due to multiple marking of pixels along the
> boundaries between adjacent tiles.

So the stencil goes in as that mask, and the image's unit square is the path whose cells are
drawn. Both halves are recomposed at the only other place in `content.rs` that can hold them, and
the graphics-state soft mask stays refused by name — two masks on one command is §11.6.5's
composition rather than a choice.

## And then the page was still blank, which is where the real defect was

`issue13561_reduced.pdf` stopped reporting and drew **nothing**, which is trap 5 exactly: a report
that goes away without the picture arriving is worse than the report. Nine cells were produced,
grouped, and every one of them landed between 129 and 512 units *below* the page.

`span` computed which tiles a path touches as `floor(low / step) ..= ceil(high / step)` — as
though the cell began at the pattern space's origin. §8.7.3.1 places the cell where its own
content stream draws it and replicates *that* at multiples of `/XStep` and `/YStep`, so tile `k`
covers `cell + k × step` and the offsets wanted are measured from the cell's own extent:

```text
first = floor((low  - cell_high) / step)
last  = ceil ((high - cell_low ) / step)
```

**This was wrong for every tiling pattern in the tree and invisible for two hundred sessions**,
because `floor` and `ceil` give a tile of slack at each end and Table 74's `/BBox` is nearly always
at the pattern's origin. `issue13561_reduced.pdf` states `/BBox [35.4 396.6 287.4 588]` against a
`/YStep` of 191.4 — 2.07 steps out — which is the first document to leave the slack.

`a_cell_far_from_the_patterns_origin_still_tiles_onto_the_path` pins it with a cell at
`[60 60 80 80]` and a step of 20: three whole steps, so the lattice is *identical* to the
origin-anchored cell's and only a reader measuring from the wrong place can tell them apart. It
fails with the old arithmetic restored.

## What it moved

```text
corpus incomplete   80 -> 78          the two documents todo 20 named
oracle agrees      851 -> 852         bug1795263.pdf, which now draws and agrees
text gate          22852 -> 22860 words of pdftotext's, still 98.2%
```

`issue13561_reduced.pdf` joined the judged set as `ambiguous` and is in
`AMBIGUOUS_IMAGE_REDUCTION`, which is what it is: a 421×320 one-bit CCITT scan drawn into 252×191
points. Two ladders agree on the geometry — `poppler` 1.228 at 576 dpi and 1.227 at 2304, `mupdf`
1.235 at 576 — and at the page's own scale ours is 1.236 against `hayro` 1.274, `poppler` 1.218,
`mupdf` 1.205 and `ghostscript` 1.091. **Ours is 0.009 from the limit and nearest of the five.**

## The habit

**A feature that stops reporting must be looked at, not counted.** The corpus gate went 80 → 78
and both documents were still wrong; only opening the page said so. That is trap 1's oldest
sentence and it earned its keep again — and the second finding was three steps beyond the first,
in a function neither document was about.
