# 585 — The mark a placement took, after the alpha had survived

§8.5.3.2's dot was drawn as nothing at 0.1 of a device pixel and the alpha it arrived with was 2.55
levels of 255. The same dot half a pixel over drew the geometry's own ink, which is §10.7.4's own
"unfavourable placement relative to the device pixel grid" and not an arithmetic floor — so the mark
is now stated as the device pixel the clause's own flooring identifies, on all three rasterisers,
because all three lost it.

Date: 2026-08-18.
Argued by: [ADR 0420](../adr/0420-the-mark-a-placement-took-after-the-alpha-survived.md).

Touched: `crates/pdf-render/src/{sub_pixel,degenerate,lib}.rs`, `crates/render-cpu/src/lib.rs`,
`crates/render-gpu/src/scene.rs`, `crates/render-quorra/src/{scene,stroke}.rs`,
`crates/render-quorra/tests/sub_pixel_coverage.rs`, `crates/render-gpu/tests/headless_gpu.rs`,
`crates/render-quorra/examples/{sub_pixel_marks,outline_stability}.rs`,
`crates/pdf-model/examples/sub_pixel_width_census.rs`, `doc/conformance/ledger.toml`,
`doc/{errata-read,QUORRA_FEEDBACK}.md`, `doc/todo/{11,_scan-conversion}.md`.

## The demand half

Session 584 left the defect named and priced, and its price was wrong in a way that only a
measurement could show. The instrument is one column: the dot ladder now draws each width with its
centre on a device pixel's **corner** and on a device pixel's **centre**, and before this round
those two read 0.0000 and 0.0078 at 0.1 on both backends. One mark, one width, one alpha, two
answers — which rules out the substitution not being reached, a library dropping the contour, and
the path degenerating, and leaves the arithmetic: a circle one device pixel across covers `π/4` of a
pixel at a centre and `π/16` at a corner, so 2.55 levels of alpha became 0.5 of a level in each of
four pixels and rounded away.

`pdf_render::point_mark` answers it with the one shape whose coverage is not a fraction, and §10.7.4
identifies it — "let i = floor( x ) and j = floor( y )". Below the width at which the widened mark
can put a level into none of the at most nine pixels it can be divided between, the mark is that
pixel at the coverage its own area implies. Nothing is promoted and no coordinate moves to a grid
line, which is what separates it from §10.7.5's stroke adjustment and from ADR 0208's refusal.

The decision went into `pdf-render` because **all three** rasterisers had lost the mark: the
processor and quorra drew nothing at 0.1 at a corner, and vello — measured here for the first time —
drew half the area at 0.2 and nothing under it. Given the true circle none of them is wrong; 0.5 of
a level in four pixels is exact coverage met by an eight-bit raster, which is why
`doc/QUORRA_FEEDBACK.md` §32 reports the finding with no ask attached.

The population was measured before the price: 97 dots over 7 of 1242 first pages, the thinnest at
0.36 of a device pixel, and 30 zero-length dashes on one document. Nothing on this disk reaches the
new construction; what the corpus does witness is that the 46 sub-pixel dots between 0.36 and 1.0
now take ADR 0290's widened form on every backend rather than on the processor alone.

The gate's rungs stopped at 0.2 and the defect was under them. They run to 0.01 now, at both
placements, on all three rasterisers, and each was confirmed to fail with the change removed.

## The spec half

§8.5.3.2's and §10.7.4's rows, and one correction neither was expecting. `spec-errata emit` over
clauses 8 and 10 before writing found **Errata Collection 3 Issue #103 over this round's own
clause** — "This rule shall apply only to zero-length subpaths" becomes "In the opaque imaging
model, this rule …", with `doc/md/` still carrying the old form and three files in this tree quoting
it. Annotated rather than changed, since the quotation gate reads the conversion. And Issue #549
strikes "shall generate an error" for "shall be ignored" where a painting operator meets an
undefined current path, which turns §8.5.3.1's one recorded departure into agreement.

## What is still owed

The body of a sub-pixel rule on `render-quorra`, below one level of alpha, which takes no
substitution there — unwitnessed, and `doc/todo/11` carries it. §8.4.3.3's cap keeps ADR 0290's
widened form deliberately: it sits on a body that lands, so no shape disappears, and concentrating
it raises the composition question `doc/todo/11` prices as "one draw rather than two".
