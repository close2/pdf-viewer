# 584 — Four renderers, four floors, and the clause states one of them

The four references draw a sub-pixel rule at four *different* device-pixel floors, all three C ones
draw exactly one whole pixel when asked for the clause's own aliased algorithm, and §10.7.5 states
the floor they draw under a condition `issue12295.pdf` does not meet — so the pale ECG traces are
this tree's anti-aliasing departure working, and what was ours was the last level of alpha, where
every substitution in `pdf_render::sub_pixel` reached zero.

Date: 2026-08-18.
Argued by: [ADR 0419](../adr/0419-four-renderers-four-floors-and-the-clause-states-one-of-them.md).

Touched: `crates/pdf-render/src/{sub_pixel,lib}.rs`, `crates/render-cpu/src/lib.rs`,
`crates/render-quorra/examples/sub_pixel_marks.rs`,
`crates/render-quorra/tests/sub_pixel_coverage.rs`, `crates/pdf-model/tests/oracle.rs`,
`doc/conformance/ledger.toml`, `doc/todo/{11,_scan-conversion}.md`.

## The demand half

Session 583 left `issue12295.pdf` named: 65 859 strokes at 0.1366 of a device pixel, drawn as a
ghost where four references draw them dark, and the question stated as *which reading is right*
rather than *how do we match them*. Both readings were defensible on their face — area is ink, or a
stroke is visible — and the round's job was to find out which the clause states.

It states the second, for the algorithm it describes. §10.7.4 read literally is aliased and a
0.1366-pixel rule is a solid run of whole pixels; ask `pdftoppm -aa no`, `gs
-dGraphicsAlphaBits=1` and `mutool draw -A 0` for that algorithm and all three answer **one whole
device pixel at every sub-pixel width**, identically. Switch anti-aliasing on and they scatter:
`poppler` and `hayro` floor at 1.0 device pixels, `mupdf` at 0.2, `ghostscript` at 0.27, the same
numbers at 72 dpi and at 576. Four rasterisers with no shared path code — `objdump -p | grep
NEEDED` says they share JPEG, JPEG 2000, FreeType, lcms2 and jbig2dec in various pairs and not one
vector rasteriser — and four different answers. The `shall` that looks like theirs is §10.7.5's,
conditioned on stroke adjustment, whose Table 52 initial value is `false`; the witness states no
`/SA`, no `/ExtGState` and no `/GS` in 375 776 bytes. `hayro`'s source, the one on this disk, says
"as required by the PDF specification" in a comment over a rule it disables for text, patterns and
Type 3 glyphs.

So the departure is licensed and the page stays where it is, with the citation beside it.

**And the page was looked at in the program rather than in the artefact**, which moved the price.
The oracle renders at 72 dpi, where those strokes are 0.1366 of a pixel and ours is the ghost. In
`target/pdf-viewer` under `Xvfb` at 900 × 1100 the page is fitted at about 1.36× and the same
strokes are 0.19 of a pixel, 48 levels of 255 — **both ECG panels legible, beat by beat**. Lighter
than the references, and a reader's page. The picture that decided nothing here is the one taken at
the resolution the gate happens to use.

**And the ladder convicted us of the one thing no reading permits**: our column is the straight line
through the origin at both resolutions and it reaches **zero** at 0.001 of a pixel. Every
substitution ADRs 0226, 0268 and 0290 built carries a mark's given-up area in the paint's *alpha*,
and an alpha is eight bits, so each has a second floor under the coverage quantum ADR 0226 answered.
`pdf_render::expressible_coverage` states a positive coverage under one level at one level — one
place, so that neither backend decides the raster's depth alone — and a 200-unit rule at 0.002 and
0.001 goes from no ink at all to ink. Nothing on this disk is a witness: the thinnest stated
sub-pixel stroke over 1005 first pages of every corpus here is 0.05 of a device pixel, and the three
documents that could reach the floor move by 0.00001 of a level, 0.0000 and nothing at scale 1, and
are byte-identical at 4×.

Two things the new seventh section of `sub_pixel_marks` printed that nobody had asked for:
`render-quorra` loses the same mark and has no substitution to floor, and §8.5.3.2's dot is drawn as
**nothing** by both backends at 0.1 as well as below it — where ADR 0290's own ladder has been
printing that row as −100% since it landed and the gate's rungs start at 0.2. The alpha floor does
not recover the dot, because its substitute is a circle inscribed in one device pixel and covers no
pixel fully; recovering it would mean snapping a mark that has a width, which ADR 0208 declines.
Both are in `doc/todo/11` with their thresholds.

## The spec half

§10.7.4's and §8.4.3.2's rows, which are the two the reading turns on. The first gains the floor and
the two residuals; the second gains what it does *not* say about a positive sub-pixel width, which
is the sentence the whole question needed and which nobody had written down. §10.7.2 and §10.7.3
were read too, as `CLAUDE.md` asks of a clause that permits, and neither reaches line width.
`spec-errata emit` over clauses 8 and 10 before writing: Errata Collection 3 touches nothing in
§8.4.3.2, §8.4.3.3–6, §10.7.3, §10.7.4 or §10.7.5 — and it *does* replace §10.7.2's "It shall be a
positive number", which `doc/md/` still carries, so that one clause has to be quoted from the
erratum rather than from the conversion.

## What is still owed

`render-quorra`'s own disappearance below one level, and §8.5.3.2's dot under 0.2 of a device pixel
on both backends. Neither is witnessed by any document on this disk, and `doc/todo/11` carries both
with the construction each would need.
