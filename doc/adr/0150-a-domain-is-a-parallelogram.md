# ADR 0150 — A domain is a parallelogram

Status: accepted, 2026-08-02. Session 180. The third defect the ambiguous ranking named, and the
first one a *picture* gave away rather than a number.

## What it looked like

`function_based_shading.pdf` is nine type 1 shadings on one page, and the oracle ranked it fourth
among undiagnosed ambiguous pages at 27.57 bounds from the nearest reference. Its side-by-side
answers the question in one look: the middle-right swatch is a **diamond** in `poppler`, `mupdf`,
`ghostscript` and `hayro`, and a **square** in ours.

The swatch is object 10, and its dictionary says why:

```
/ShadingType 1 /ColorSpace /DeviceRGB /Domain [0 1 0 1]
/Matrix [85 85 -85 85 515 382] /Function 19 0 R
```

`[85 85 -85 85]` is a rotation by 45 degrees with a scale. §8.7.4.5.2 says what that means, and
the sentence is one this reader had implemented half of:

> The transformation matrix ( Matrix ) then maps the domain rectangle into a corresponding
> rectangle **or parallelogram** in the target coordinate space. Points wi thin the shading's
> bounding box ( BBox ) that fall outside this transformed domain rectangle shall be painted with
> the shading's background colour ( Background ); if the shading dictionary has no Background
> entry, such points shall be left unpainted.

The domain *rectangle* was read: `function_based` samples the colour function over it onto a
128×128 grid, and `Shading::transform` carries the `/Matrix`, so the colours land in the right
places. What was missing is the second half — where the shading stops. `render-cpu` builds the
grid into a `tiny_skia::Pattern` with `SpreadMode::Pad`, which is correct for the *interpolation*
and says nothing at all about the shading's extent, so the padded edge colour ran out to fill the
clip. The clip was the swatch's own `re W n` square, and a square is what came out.

## The fix, and where it belongs

An extra clip, composed by the interpreter from the domain's four corners under
`Shading::transform`, exactly as Table 77's `/BBox` already is. `Interpreter::domain_clip`, applied
in `paint_shading` for the `sh` operator and in `paint_clip` for a shading *pattern*, so both
routes to a type 1 shading get it.

The interpreter rather than the backend, for the reason `bbox_of`'s comment already gives about
`/BBox`: where a shading stops is a clause about the shading's placement, not a property of the
gradient's colours, and putting it here means both backends inherit it and the display list carries
it. It is exact, too — the parallelogram is antialiased by the ordinary fill machinery, where the
alternative (a transparent border row around the sampled grid) would have softened every type 1
shading's edge by half a sample.

Nothing changes for the other shading types. An axial or radial shading already states where it
stops through `/Extend`, which the ramp carries as transparent stops (§8.7.4.5.3), and a mesh
through its triangles. `ShadingKind::Sampled` is produced by `function_based` and by nothing else,
which is what makes the match arm a type test.

`/Background` remains unimplemented, and **the first version of this paragraph said it was
*refused*, which is false**: nothing reads Table 77's entry and nothing reports it. What the
ledger's §8.7.4.3 row says is what is true — it is unimplemented, two corpus documents write one,
and it applies only "when the shading is used as part of a shading pattern, not when painted
directly with the sh operator". So the clip composed here leaves those points unpainted, which is
the clause's branch for a shading with **no** `/Background`, and a shading that states one gets
the same treatment without saying so. Corrected in the hundred-and-eighty-first session, which met
the entry on `issue13372.pdf`.

## What it cost

Corpus unchanged (974, 0 unopenable, the same 74 incomplete), text unchanged at 98.2%, dates
unchanged, and **no oracle verdict moved** — including `function_based_shading_cmyk.pdf`'s two
contradicted pages, whose numbers are identical before and after.

`function_based_shading.pdf page 1` went from 27.57 bounds from the nearest reference to under 6.

## What is left on that page, and why it is settled anyway

It stays `ambiguous`, on one swatch, and the specification decides that one too — by hand.
Object 11's function is object 20, whose whole program is
`{ 4 mul floor exch 4 mul floor add 2 mod }`: for inputs (x, y) that is
`(floor(4x) + floor(4y)) mod 2`, a four-by-four checkerboard, and §7.10.5 defines every operator
in it. We draw the checkerboard; so do `mupdf` and `hayro`. **`poppler` draws the swatch solid
black and `ghostscript` draws it flat mid-grey** — the average of a checkerboard, which is what
sampling a discontinuous function onto a smooth mesh produces. Two references failing in two
different ways is trap 9's shape, and it is why the verdict cannot move.

`AMBIGUOUS_FUNCTION_SAMPLED_BY_A_REFERENCE` carries that, and it is §3a's first shape: the clause
determines the answer completely, and the check needs no reference at all.
