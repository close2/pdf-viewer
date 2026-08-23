# 688 — The wash painted, and the witness that witnessed nothing

A pixels round on §8.7.4.3 Table 77's `/Background`, taken from `doc/todo/17`. The paint landed on
all three backends and all four shading kinds; three of the four prices the todo file carried were
wrong, and so was the sentence three documents used to say what its headline witness draws.

Date: 2026-08-23.
ADR: [0529](../adr/0529-the-wash-painted-and-the-three-prices-that-were-wrong.md).

Touched: `crates/pdf-render/src/shading.rs` (`Shading::background`, a new `ShadingRaster`),
`crates/pdf-render/src/lib.rs`, `crates/pdf-render/src/repeat.rs`,
`crates/pdf-model/src/shading.rs` (a new `background_of`, a named `Built` for the cache),
`crates/pdf-model/src/content/pattern.rs` (`ShadingDefinition::paints_background`, the report's new
condition, the domain clip's two branches), `crates/render-cpu/src/{lib,shading}.rs`,
`crates/render-gpu/src/{scene,shading}.rs`, `crates/render-quorra/src/scene.rs`,
`crates/test-scenes/src/lib.rs` (a new `shading_background` scene), seven fixtures in
`crates/pdf-model/tests/shadings.rs`, one comparison apiece in `headless_quorra.rs` and
`headless_gpu.rs`, six backend test fixtures for the new field,
`crates/pdf-model/tests/oracle.rs` (three page lists and three notes),
`doc/conformance/ledger.toml` (§8.7.4.3 `partial` → `implemented`, §8.7.4.5.2, §11.6.7),
`doc/state-of-play.md`, `doc/todo/README.md`, `doc/todo/01`,
`doc/todo/17-a-shadings-background.md` (deleted), the ADR and this file.

## The clause first, which is what the briefing asked for and what paid

§8.7.4.3's Table 77 says the wash fills "those portions of the area to be painted that lie outside
the bounds of the shading object", and only for a pattern. ADR 0452 had that. What it did not have —
and what decided the whole implementation — is **§11.6.7**, which states the construction outright:
the pattern's implicit transparency group "shall be filled with the specified background colour
before the sh operator is invoked", the group is a *knockout* one, and its colour and shape then
become the object's. So the wash and the shading are one painting operation, with one coverage at the
path's edge and one §11.6.4.4 alpha.

`doc/todo/17` had reached the same construction by an argument about antialiased coverage. The
argument is right and was never needed. That is principle 5's own decaying-silence rule arriving
again: before recording that a clause states a requirement and no construction, read the *titles*
around the subject in `doc/md/`. §11.6.7's title is *Patterns and transparency*.

## What was built, and the price that was not owed

`pdf_render::Shading` gains `background: Option<Color>`, resolved through the shading's colour space
by `pdf_model::shading` and set only where a shading arrives through a `/PatternType 2` pattern. A
new `pdf_render::ShadingRaster` answers, per device pixel, the shading's colour where the geometry
has one and the background where it has none — for all four kinds.

All three backends then draw that one raster through the shape they were going to fill. **No backend
needed a new lane**: `fill_with_raster`, `fill_radial` and quorra's `Paint::Mesh` are all "a
device-resolution RGBA raster confined to the path", built for §8.7.4.5.5's mesh and §8.7.4.5.4's
cone. The todo file's largest single item — an upstream ask for quorra's gradient lane — is not owed,
and nothing was written into `doc/QUORRA_FEEDBACK.md` because there is nothing to ask for. Two of its
other three rows were wrong outright; the ADR has the table.

Still reported, on a condition rather than a subject: a *stroking* selection, because the raster lane
is a fill's door in all three backends, and an array whose length the colour space cannot use.

## The witness that witnesses nothing

`issue13372.pdf` was this entry's headline witness in ADR 0452, `doc/todo/17` and §8.7.4.3's ledger
row, and all three said the page's corners project outside `[0, 1]` on the shading's axis so the
stencil beyond the band is cyan and unpainted here. The area to be painted is not the page: the
stencil is placed at exactly `(90, 108)`–`(522, 684)` and `/Coords [90 108 522 684]` is that
rectangle's diagonal, so every point of it has `t` in `[0, 1]` and the wash has zero area.

Measured by rendering each witness both ways in one sitting, patch out and patch in:

```text
                    pixels changed   of the page   largest change
  issue13372.pdf            26 690         5.33%        1 level     (the axial leaving tiny-skia's
                                                                     gradient, not the wash)
  issue18816.pdf               149         0.03%      15 levels     (the wash, on one raster row)
```

So the entry is implemented on the standard's evidence and on fixtures, and the corpus never could
have ranked it — `CLAUDE.md`'s two denominators, exactly.

## The page that moved and was not ours

`function_based_shading_cmyk.pdf` page 2 left the contradicted list in the same oracle run. It states
no `/Background`, and our raster for it is **byte-identical** across the change — checked with `cmp`
on two renders taken either side of the patch, which is the only thing that separates "this round
moved a page" from "a page moved this round". What moved is the consensus: the closest two references
now miss each other by 29.06 against a page bound of 1.00. It is in `AMBIGUOUS_DEVICE_CMYK_CONVERSION`
now, and page 1 of the same file is still contradicted on the same mechanism.

`issue13372.pdf` page 1 re-entered the comparison and went back to `AMBIGUOUS_IMAGE_REDUCTION`, on
the halftone diagnosis it always had.

## Instruments

The whole of `doc/todo/02` §2, plus `overstated`, `overtaken`, `tables`, `pointers`, `quoted` against
this round's own oracle log — none of which names a note this round wrote — and `doc/todo/00` step 7
over all 769 ambiguous pages. §5's binaries rebuilt and installed. Every gate's number is in the run;
none of it is in this file, which is ADR 0281's rule.
