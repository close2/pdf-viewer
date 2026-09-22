# 1211 — A frame measured, stage by stage, and a mesh raster divided

The performance round, on `doc/todo/36`: a picture every refresh, 60 Hz the floor and 120 Hz the
target — 8.333 ms. The cadence around a late frame was built and argued long ago (ADRs 0383–0391);
what had never been measured was the frame itself.

## Measured

`crates/render-raster/examples/frame_budget.rs`, and `tools/state.sh frame` under it: three page
classes — dense text, patch meshes, one five-megapixel photograph — in three rows each, the page
turned to, the same frame asked for again, and the page at twice the magnification, split into
interpretation, this tree's scene walk, raster's encode, the transfer, the device's passes and
what is left. ADR 1260 has the table and the two mistakes that came before it: a page turn and a
zoom step are drawn on **different coverage lanes**, and interpretation has to be timed against the
**font cache a page turn has** — which is the difference between 12.45 ms and 1.37 on the text
page, in the stage the question was about.

What it establishes: a repaint always fits inside a refresh, a page turn fits on none of the
three, the largest stage differs per page class, and the device's own passes are a few per cent of
every row — so every remaining item is a host item.

## Taken

The largest of them that is this tree's and that a round could reduce: the mesh page's scene walk,
**8.22 ms of an 18.04 ms zoom step**. `MeshRaster`'s rows now divide across rayon's pool and it is
**3.98**. The division is byte-identical by construction — a mesh is point-sampled, so a band is
not a boundary in the arithmetic the way ADR 0138's antialiased strips were — and
`both_arms_of_the_division_paint_the_same_bytes` holds it there, calibrated against the defect.
ADR 1259 has the A/B, four alternating pairs in one sitting, and why the obvious floor (the image
reducer's 65 536 samples) made the change do nothing.

## Asked and left

`doc/QUORRA_FEEDBACK.md` §52 carries the two costs that are raster's, with the table under them:
the CPU-lane encode of a first sight (77% of a refresh), and an image restaged per placement.

Left, with its callgrind attribution written into `doc/todo/36` so the next round does not have to
find it again: the photograph's 78.68 ms of interpretation — two thirds `zune-jpeg`'s, about a
sixth in `pdf-model`'s `image.rs`, whose `frame_as_defined` scans every `DCTDecode` codestream end
to end for a marker nearly no file states. That file belonged to no round of this batch.
