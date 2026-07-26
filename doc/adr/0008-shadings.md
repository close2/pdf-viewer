# ADR 0008 — Seven shading types, four display-list kinds

Status: accepted, 2026-07-26. All seven types are implemented, on both backends.

## Context

PDF defines seven shading types, and they are not seven variations on one idea. Types 2
and 3 are gradients along a line and between two circles. Type 1 is an arbitrary function
of two variables. Types 4 to 7 are meshes: triangles with a colour at each corner, and
Coons and tensor patches that reduce to them.

All seven share a colour model — a PDF *function* mapping a parameter to components in
some colour space — and reach the page through two different routes: the `sh` operator,
which floods the clip region, and a shading *pattern* set as the fill or stroke colour.

A survey of the pdf.js corpus (974 documents) shows the distribution is extremely uneven:

| | occurrences | documents |
|---|---|---|
| type 2, axial | 1698 | 47 |
| type 3, radial | 49 | 8 |
| type 1, function-based | 18 | 2 |
| types 4–7, meshes | 28 | 8 |

## Decision

**The display list carries four kinds, not seven.** Axial and radial stay distinct because
both rasterisers implement them natively — and both take a radius for *each* circle, which
is exactly PDF's model rather than the common single-circle simplification. Type 1 reduces
to a grid of samples. The four mesh types all arrive as triangles, since that is what they
all describe.

That both `tiny-skia` and Vello express these in the same terms, without translation, is
the evidence that the neutral form is the right one — the same argument ADR 0002 makes for
the display list as a whole.

Nothing is lost by that grouping except the type's number, which no backend needs.

**Colours are resolved before the display list**, as `pdf_render::Color` already required:
the function is evaluated and the colour space applied upstream, so a backend never sees
either. This also keeps the display list plain data — it can be compared, shared and sent
across a process boundary, none of which a stored closure allows.

**Sampling resolution is a deliberate compromise in one place.** A ramp is 256 entries; a
type 1 grid is 128×128. Everything else in the display list is resolution-independent, and
this is the exception, taken because a PDF function cannot live below `pdf-model` and a
type 1 shading has no closed form to carry instead.

## `/Extend` has no equivalent and needed inventing

Where a shading does not extend, it paints **nothing** beyond that end. That is not the
same as painting the end colour: it is the difference between a band across part of a shape
and a wash over all of it, and it is invisible in a rendered page unless you know what the
document asked for.

No spread mode expresses it. `Pad` repeats the end colour, `Repeat` and `Reflect` tile. So
a non-extended end gets a fully transparent gradient stop at the very edge of the ramp,
carrying the neighbouring colour so no fringe appears as it fades; `Pad` then repeats
*transparency*. The cut-off is a gradient a twentieth of a percent of the axis wide rather
than a hard edge, which is far under a pixel on any real page.

## Pattern space is not user space

A shading pattern is positioned by its `/Matrix` relative to the page's **default**
coordinates, not to the transform in force when the pattern is used. Getting this wrong
moves every gradient on the page by whatever the current transform happened to be, and the
result still looks like a gradient — so a test pins that a pattern fill and `sh` put the
same colour at the same point.

## Consequences

Every shading in the `doc/` corpus now draws; the only thing still reported there is a soft
mask, which is transparency rather than shading. Across the pdf.js corpus **all 1793
shadings build**, mesh types included.

PDF functions and colour spaces arrived with this and are worth more than the shadings:
functions are needed by soft masks, transfer functions and `Separation` spaces, and the
colour-space work makes `Separation`, `DeviceN`, `Indexed` and `Lab` exact everywhere
rather than only in shadings.

Both backends draw them, and three scenes in the headless suite hold them to agreement.
That check is what makes the GPU work verifiable at all: the CPU backend's colours are
pinned against known values and against poppler, so agreement carries that verification
across rather than restating it.

## Meshes needed measurement, not reasoning

Neither rasteriser has Gouraud shading, so both subdivide each triangle until its corner
colours agree to within one part in five hundred and fill it flat. Subdivision is by
quarters rather than halves: repeatedly splitting one edge produces slivers, and slivers
rasterise with seams.

Abutting triangles then do not tile exactly. Coverage along a shared edge does not sum to
one and the backdrop shows through — as nine isolated white pinholes on the specification's
own Coons page under `tiny-skia`, and as a hairline along *every* shared edge under Vello,
which antialiases each edge.

A comment written here first claimed Vello needed no repair because it composites analytic
coverage. That was wrong, and only measuring showed it. Growing each triangle so neighbours
overlap fixes both, and sweeping the amount against the CPU-versus-GPU agreement test gave
mean errors of 12.2 at no overlap, 4.2 at 0.35 px, 1.8 at 0.7 px, and within tolerance at
0.8 px — with the CPU backend antialiasing its triangles too, which it can do safely
because overlapping edges sum to full coverage where abutting ones do not.

Both backends use the same number for the same reason. They have to: two different
approximations of one thing would leave them disagreeing for a reason unrelated to either.

## Tiling patterns are expanded, not painted

A tiling pattern (`/PatternType 1`) is a content stream rather than a paint, so it never
becomes one. The filled path becomes a clip and the cell is replayed once per tile position
inside it. No display-list concept is added, no backend learns what a pattern is, and the
result stays resolution-independent because the cell is real geometry rather than a
rendered image. Tile counts are bounded and reaching the bound is reported.

Two things about tiling patterns are easy to get wrong and invisible afterwards. `/XStep`
and `/YStep` are allowed to differ from the cell's bounding box — that is how a pattern
tiles with space around each figure — and the phase comes from the pattern matrix relative
to the *page*, not to the transform in force at the fill. Both have tests, because both
still produce something that looks like a pattern.

## What is not done

**Sampled shadings on the GPU.** A type 1 shading becomes a grid of samples, which
`tiny-skia` takes as a pattern; the Vello path reports it instead. Eighteen occurrences in
two documents of 974, and reported rather than silently wrong.

**Colour management.** Our `DeviceCMYK` conversion is the uncalibrated one, and on a
CMYK mesh that is a visible saturation difference against poppler even where the geometry
matches exactly. That is a pre-existing gap `colour.rs` documents, not a shading one, but
meshes are where it shows most.
