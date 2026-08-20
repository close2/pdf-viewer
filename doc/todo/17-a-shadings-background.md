# The wash a shading pattern asks for around itself

Status: **found in the six-hundred-and-sixteenth session, reported in the same one, unpainted.**
Priority: 17 — a defect: a mark the file asks for that this tree does not make. It is *reported*
since the round that found it, so it is no longer silent; what is owed is the paint.
Corpus: **2 documents of the 974**, and **2 of the 1249** this project measures over —
`issue13372.pdf` and `issue18816.pdf`, both of which use the shading as a `/PatternType 2`
pattern, which is the only place the entry means anything.
Clauses: §8.7.4.3 (Table 77's `/Background`), §8.7.4.5.2 (the same entry stated again for a type 1
shading), §8.7.4.2 (the `sh` the entry exempts)
Code: `crates/pdf-model/src/shading.rs` (`background_components`, which reads it),
`crates/pdf-model/src/content/pattern.rs` (which reports it), `crates/pdf-render/src/shading.rs`
(where a `Shading` would carry it), `crates/render-cpu/src/shading.rs`,
`crates/render-quorra/src/scene.rs`, `crates/render-gpu/src/scene.rs`

## What the clause asks for

Table 77, verbatim:

> ( Optional ) An array of colour components appropriate to the colour space, specifying a single
> background colour value. If present, this colour shall be used, before any painting operation
> involving the shading, to fill those portions of the area to be painted that lie outside the
> bounds of the shading object. NOTE 1 In the opaque imaging model, the effect is as if the
> painting operation were performed twice: first with the background colour and then with the
> shading. The background colour shall be applied only when the shading is used as part of a
> shading pattern, not when painted directly with the sh operator.

§8.7.4.5.2 states the type 1 case again, and in a form that names the geometry outright:

> Points wi thin the shading's bounding box ( BBox ) that fall outside this transformed domain
> rectangle shall be painted with the shading's background colour ( Background ); if the shading
> dictionary has no Background entry, such points shall be left unpainted.

Two things follow that bound the work. The entry is a `shall`, so this is a defect rather than a
permission; and its condition is the *pattern*, so `sh` owes nothing and a report raised on `sh`
would be noise on every page that paints a shading directly.

## The witnesses, and why they are witnesses

`issue13372.pdf` states `/Background [0 1 1]` on an axial shading with
`/Coords [90 108 522 684]` and **no `/Extend`**, so the default `[false false]` applies and the
shading's bounds are the band between the two lines perpendicular to that axis through its
endpoints. The pattern fills a CCITT stencil over a 595 × 842 page (ADR 0151), and the page's
corners project outside `[0, 1]` on that axis — so every marked cell of the stencil beyond the band
is cyan in the document and unpainted here.

`issue18816.pdf` states `/Background [.09226 .09003 .08394]` on a `/ShadingType 6` Coons patch
mesh, again through a `/PatternType 2` pattern. There the bounds are the union of the patches, and
the background is the wash around them.

Both are in the pdf.js 974. The other four `doc/corpora/` submodules state no shading
`/Background` at all: `witness_census` over the whole 1251 finds five documents stating the *name*
and three of those are an optional content group called `Background` and a `/PieceInfo`
`/Private /Background`, neither of which is Table 77's.

## Why the NOTE's construction is not the implementation

The obvious reading — emit a solid fill of the same path in the background colour, then the
shading command over it — is the NOTE's own, and the NOTE says which model it holds in: *the
opaque imaging model*. On this device it is wrong twice.

- **At the path's boundary.** Two marks with the same anti-aliased coverage `c` do not compose as
  one mark: the backdrop keeps `(1 − c)²` where the clause leaves `1 − c`, so a background-carrying
  pattern would gain a one-pixel fringe of the background colour around every path it fills. This
  is §10.7.4's departure (2) at its maximum, because the two marks are *coincident* rather than
  merely abutting — `doc/todo/11` item 5 and ADR 0308 are the same arithmetic on a seam.
- **Under a constant alpha or a blend mode.** §11.6.4.4's `ca` applies to the painting operation,
  and performing it twice applies it twice: inside the bounds the result is
  `ca·S + (1 − ca)·(ca·B + (1 − ca)·D)` where the clause's single operation gives
  `ca·S + (1 − ca)·D`.

So the construction that is exact is the other one: **the shading's paint answers the background
colour where it would otherwise answer nothing**, which is one painting operation, one coverage and
one alpha, and which is what the clause describes in effect — the area to be painted ends up
covered by the background outside the bounds and by the shading inside them.

## What that costs, by backend and by shading kind

`pdf_render::Shading` gains a `background: Option<Color>`, resolved through the shading's own
colour space by `pdf_model::shading` exactly as its ramp already is, and set **only** where the
shading arrived through a pattern. `with_alpha` scales it, `is_opaque` accounts for it. Then:

| kind | processor (`render-cpu`) | quorra | graphics device (`render-gpu`) |
|---|---|---|---|
| axial, radial (gradient path) | the transparent stop `stops()` places at a non-extended end becomes a background-coloured **opaque** stop; `SpreadMode::Pad` then repeats the background rather than transparency. A few lines | `quorra_scene::ShadingKind::Axial`/`Radial` carry `extend` and no background — an **upstream ask**, or the same fallback the radial cone path already takes | as the processor, through its own gradient |
| radial cone (`fill_radial`) | `pdf_render::RadialRaster` already writes a transparent pixel where `blend_parameter` finds no admissible root; it writes the background there instead | the same raster, uploaded — quorra draws `RadialRaster`'s bytes already | the same |
| mesh (`fill_mesh`) | `pdf_render::MeshRaster` leaves every pixel no triangle covers transparent; the background is its clear colour | the same raster | the same |
| sampled (type 1) | the outside is a **clip** rather than a colour — `Interpreter::domain_clip` removes it — so the background is a second fill of the region between Table 77's `/BBox` (or the path) and the domain parallelogram, which is the one kind where the NOTE's two-operation shape is exact because the two regions are disjoint | `quorra_scene::Paint::Function` **already has a `background` field**, which is where the pricing starts | as the processor |

The largest single item is quorra's gradient lane, and it is not this tree's to change. Three
answers exist and the choice is the round's: ask upstream (`QUORRA_FEEDBACK.md`), send a
background-carrying axial or radial down the raster lane the cone case already uses, or leave that
one lane reporting while the other two draw — which would be a backend disagreeing with a backend,
which is what the cross-backend gate exists to refuse.

## What has to be true before it is taken

- **A fixture per kind, hand-built.** Two corpus documents cover an axial and a Coons mesh; nothing
  in any population here states a background on a radial or a type 1 shading, so those are trap 8's
  case and want a fixture that asserts the colour at a point outside the bounds.
- **The cross-backend gate on all three**, at page scale and at 4×, because the whole point of
  putting the colour in `pdf-render` is that no backend decides it alone (trap 2).
- **`doc/todo/00`'s step 7 re-run**, because this adds ink to two pages and the ink sweep is what
  sees a page drawn light.
- **`issue13372.pdf` is `ambiguous` and stays a question**: its own entry in `doc/todo/00` is about
  a halftone screen, and adding the wash will move its numbers without settling the ranking that
  put it there.
