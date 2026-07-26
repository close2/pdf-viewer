# ADR 0008 — Seven shading types, four display-list kinds

Status: accepted, 2026-07-26. Mesh types 4–7 are **not implemented**; they are reported.

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
both rasterisers implement them natively — `tiny-skia`'s radial gradient takes a radius for
*each* circle, which is exactly PDF's model rather than the common single-circle
simplification. Type 1 reduces to a grid of samples. The four mesh types would all arrive
as triangles, since that is what they all describe.

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
mask, which is transparency rather than shading. Across the pdf.js corpus, 1765 of 1793
shadings build, and all 28 failures are the mesh types.

PDF functions and colour spaces arrived with this and are worth more than the shadings:
functions are needed by soft masks, transfer functions and `Separation` spaces, and the
colour-space work makes `Separation`, `DeviceN`, `Indexed` and `Lab` exact everywhere
rather than only in shadings.

The GPU backend does **not** draw shadings yet and reports them. That is deliberate: the
comparison harness excludes a page a backend says it cannot draw, so the two backends stay
honestly different rather than quietly so. Vello has native gradients, so axial and radial
should map across much as they did for `tiny-skia`.

## What is not done

**Mesh shadings (types 4–7).** They need a bit-packed vertex stream reader and, for types 6
and 7, Coons and tensor patch subdivision. `pdf_render::Triangle` already carries the
representation and the subdivision helpers the backends would need — `is_flat`,
`subdivide` — because the display-list side of the design is settled; only the reading and
the drawing are missing. They are 1.6% of shadings in the wild and are reported as
unimplemented, so they are visible rather than silently blank.

**Tiling patterns (pattern type 1)** are a content stream drawn repeatedly, not a shading,
and are a separate piece of work. They are commoner than every mesh type combined — 955
occurrences across 35 documents — and are the more valuable of the two.
