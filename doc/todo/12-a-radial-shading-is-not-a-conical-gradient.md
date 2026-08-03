# A radial shading is not a two-point conical gradient

Status: **diagnosed, with the arithmetic; not started.**
Priority: 12 — a defect, in a clause that states its algorithm exactly
Corpus: `radial_gradients.pdf` pages 4 and 5, 8 cells of 24 measurably wrong
Clauses: §8.7.4.5.4, Table 80
Code: `crates/render-cpu/src/shading.rs`, `crates/render-gpu`, `crates/render-quorra`,
`crates/pdf-render/src/shading.rs`

## The page

`radial_gradients.pdf` is a test sheet: twenty-four radial shadings in a grid, four `/Extend`
combinations across each of six geometries. Pages 4 and 5 sit on §3a's ranking at 2.70 and 2.74
bounds from the nearest reference **and within 0.01 of the furthest**, which is the
everybody-against-us shape step 1 says to prefer.

Four renderers draw a filled disc with a cone on it. We draw only the crescent between the two —
the disc's interior is missing.

## The arithmetic, and it is decisive

The cell is `/Coords [511 489 25 431 489 60] /Extend [true false]`. So

```text
c(s) = (511 − 80s, 489)      r(s) = 25 + 35s
```

The circles are 80 apart and their radii differ by 35, so neither contains the other: §8.7.4.5.4's
NOTE 3 cone. `/Extend [true false]` continues the family below s = 0 until r(s) = 0 at
s = −25/35 = −0.714, and stops it at s = 1.

Take the centre of the ending circle, **P = (431, 489)**. The clause:

> if a point lies on more than one blend circle, its final colour shall be that of the last of the
> enclosing circles to be painted, corresponding to the greatest value of s

|P − c(s)| = 80|s − 1|, and r(s) = 25 + 35s, so the blend circles through P are

| root | s | r(s) | admissible? |
|---|---|---|---|
| s ≤ 1 | **0.478** | 41.7 | yes |
| s > 1 | 2.333 | 106.7 | no — `/Extend[1]` is false |

**The point is painted, at t = 0.478.** We paint nothing there.

## Why

`render-cpu` hands `ShadingKind::Radial` to `tiny_skia::RadialGradient`, which is the
SVG/Canvas **two-point conical gradient**. The two constructions agree on the common cases and
not on this one: a conical gradient solves for one root and clamps it with its spread mode, where
the clause says *take the greatest admissible root, and where the greatest is out of range fall
back to the other one*. Our `/Extend` emulation — a transparent stop just inside each end — then
turns "out of range" into "paint nothing", which is right for the axial case and wrong here.

The other two backends build their own gradients from the same `ShadingKind::Radial` and agree
with the CPU one closely today (`render-quorra/tests/corpus.rs` is at 912/44/1), so this is a
decision all three inherit from the same place.

## The fix, and where it goes

**`pdf-render`, for `MeshRaster`'s reason**: "neither rasteriser has the primitive and a second
copy would drift" (ADR 0051). A radial shading is the same shape of problem one type over — three
gradient implementations, none of them §8.7.4.5.4's.

The evaluator is small and exact. For a point P, `|P − c(s)| = r(s)` squares to a quadratic in s
whose coefficients are `(dx² + dy² − dr²)`, and the answer is the greatest root with `r(s) ≥ 0`
that `/Extend` admits, with the degenerate linear case when the leading coefficient is zero
(equal radii — the strip case). NOTE 1's "the blend circles continue as far as that s value for
which r(s) is large enough to encompass the shading's entire bounding box" is the upper extension
and needs the target's extent, which is why this belongs beside the rasteriser rather than in the
display list.

Then each backend draws it the way it draws a mesh: `render-cpu`'s `fill` already special-cases
`ShadingKind::Mesh` and calls `shading::fill_mesh` with the path and the surface, so the same door
is open. The likely shape is a raster at the *device* resolution of the shape's bounds, used as a
pattern — which costs one quadratic per device pixel, comparable to a gradient shader, and is
exact.

## A fourth thing the mesh precedent does not cover, found by looking

`render-cpu` special-cases `ShadingKind::Mesh` **inside `fill`**, where it has the shape and the
surface. A *stroke* with a mesh paint therefore falls through to `self.paint`, gets no shader and
comes back `UnsupportedPaint` — which is right for a mesh, because nothing here can express one
as a paint, and would be a **regression** for a radial, which has a working gradient today.

So the raster has to arrive as a `tiny_skia::Shader` from `paint` rather than as a special case in
`fill`, which is what `sampled_shader` already does for type 1 — and that runs straight into trap
2. `fill` and `stroke` hand `paint` a *path-space* transform, because `tiny-skia` applies the
drawing transform to the paint as well as to the shape; a raster built in **device** space would
be transformed a second time. Its pattern transform has to be the device translate composed with
the inverse of the path's, and getting that wrong mirrors every gradient on the page, which is
exactly what trap 2 is about.

`paint` also does not know the target's extent, which a device-resolution raster needs.
`MeshRaster::build` takes `width` and `height`; threading those through `paint` is a signature
change in the one function every fill and stroke goes through.

None of this is hard. It is four intertwined changes across three backends with trap 2 sitting in
the middle, which is more than a session should start at its end.

## What has to be settled first

- **Whether to keep the fast path.** A concentric or nearly-concentric shading is the common case
  and every gradient implementation gets it right; a rule for when to use which needs an argument
  and a measurement, not a guess. The alternative — always exact — is simpler and its cost is one
  benchmark away from being known.
- **All three backends at once, or the cross-backend gate pays.** Fixing only the CPU one would
  trade agreement with the references for disagreement with `render-quorra`, which is the wrong
  trade to make silently.
- **What happens at the edge.** A per-device-pixel evaluation gives a hard boundary where the
  shading stops. §10.7.4 asks for a hard edge, and this tree anti-aliases elsewhere on purpose
  (ADR 0025); doing the opposite here should be a sentence in an ADR rather than an accident.

## Why it is not started

The questions above are a design, and `CLAUDE.md`'s first principle says a thing that cannot be
done properly now is not started now. What is *done* is the diagnosis: the clause, the arithmetic
at a named point, the reason the current construction cannot express it, the place the fix
belongs, and — from the two-hundred-and-tenth session, which opened the code to size it — the
four changes it actually takes and the trap in the middle of them.

**The mesh precedent is a precedent for the evaluator and not for the wiring**, which is the
thing that was not obvious until somebody read `fill` and `stroke` side by side.
