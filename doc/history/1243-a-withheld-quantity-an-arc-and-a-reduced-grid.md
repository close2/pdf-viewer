# 1243 — five partial leaves re-read against the tree: two rows closed, one departed, two narrowed

2026-09-23. ADRs 1323 (a list box's selection highlighted at a chosen colour) and 1324 (an arc cut
as cubics within a stated bound; a matte's mask carried onto a reduced JPX grid).

- **§8.9.6.2** stays `partial`, narrowed. ADR 1287 had already answered `QUORRA_FEEDBACK` section
  39: `filtered_edge_colour` prints 0 on all three backends. A stencil is already premultiplied (its
  alpha is 0 or 255), so the fix was the shader stopping its multiply after the filter. New test:
  `masked_image_edge.rs`, a magnified stencil over white keeps a red channel of 255 at four
  non-integer scales; putting the late multiply back makes it fail at 245. Still left: a stencil
  painted through a pattern under a graphics-state soft mask (`content/image.rs`, 1241's file).
- **§12.5.6.23** stays `partial`. Arcs in a stroke's outline are now cut. The test holds each moved
  pixel to the clause's own geometry, worked out analytically. The redacted page is within one
  step of it; `render-cpu`'s own stroker is up to eight steps off. A reduced JPX stays refused:
  writing it back would resample everything outside the region too.
- **§12.7.4.3** `partial` → `departed`, for rich-text formatting alone. A singular `/DA` `Tm` is
  the clause carried out: the translation is the processor's to choose, and the producer's matrix
  draws nothing. Its test now also asserts there is no ink.
- **§11.6.5.2** `partial` → `implemented`. `alpha_on_grid` carries the mask onto the reduced grid.
  The fixture is a 4800² `opj_compress` codestream (8.9 KB, `tests/jpx/`); the page draws 191,
  against 239 without the `/Matte`, and the old pairing fails the test. The same function now also
  covers a mask that decoded on another grid, which used to be paired unchecked.
- **§12.7.5.4** `partial` → `implemented`. A72's near side: the clause names the selection and
  withholds how it looks. §12.7, §12.7.4 and §12.7.5 go to `departed` with their subclauses, as
  the aggregate gate requires.

Gates: `pdf-transform --test gate` passed. `raster_golden` moved 9 rows and `render-raster` corpus
added one differing page (`ContentStreamNoCycleType3insideType3.pdf`). Neither is this round's:
none of those files has an AcroForm, an `/SMask` or an object stream, so nothing changed here
reaches them. The Type 3 page is the interpreter change in `content/text.rs`, which is 1241's.
