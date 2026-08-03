# ADR 0171 — A radial shading is not a gradient, and the difference is one quadratic

Status: accepted, two-hundred-and-thirty-second session.
Supersedes nothing. Closes `doc/todo/12-a-radial-shading-is-not-a-conical-gradient.md`.

## Context

ISO 32000-2 §8.7.4.5.4 defines a type 3 shading as a *family* of circles indexed by a
parameter `s`, with centre `c(s) = c0 + s(c1 − c0)` and radius `r(s) = r0 + s(r1 − r0)`, and
states how they compose:

> Conceptually, all of the blend circles shall be painted in order of increasing values of
> s , from smallest to largest. … The painting is opaque, with the colour of each circle
> completely overlaying those preceding it. Therefore, if a point lies on more than one blend
> circle, its final colour shall be that of the last of the enclosing circles to be painted,
> corresponding to the greatest value of s .

and `/Extend` decides which of those circles exist at all:

> If the first of the two elements is true , the shading shall be extended beyond the defined
> starting circle to values of s less than 0.0; if the second element is true , the shading
> shall be extended beyond the defined ending circle to s values greater than 1.0 unless radii
> r 0 and r 1 in the Coords array are both zero.

All three backends handed the shading to a two-point conical gradient —
`tiny_skia::RadialGradient`, `peniko::Gradient::new_two_point_radial`, quorra's own
`ShadingKind::Radial` — and every one of those solves for **one** root and clamps it with a
spread mode. `/Extend false` was emulated by a transparent stop just inside the ramp, which
turns "this root is out of range" into "paint nothing".

The clause says something the gradient cannot: where the greatest root is one `/Extend`
refuses, **the other root still paints**. `radial_gradients.pdf` pages 4 and 5 are the
witness — twenty-four shadings in a grid — and on the cone cells four reference renderers drew
a filled disc with a cone on it while we drew only the crescent between the two circles. Ink
66.05 and 67.19 against 70.55 to 73.94, and worst tile 108.49 against a bound of 40.

Diagnosed in the two-hundred-and-sixth session with the arithmetic at a named point, and not
started for twenty-six because the wiring had four intertwined parts and trap 2 in the middle
of them.

## Decision

**Solve the quadratic and take the greatest admissible root**, in `pdf-render`, and let all
three backends draw the same bytes.

`|p − c(s)| = r(s)` squares to `a s² + b s + c = 0` with
`a = |Δc|² − Δr²`, `b = −2(p·Δc + r0·Δr)` and `c = |p|² − r0²` (with `p` and `Δc` relative to
`c0`). `pdf_render::blend_parameter` returns the greatest root that is *admissible* — `r(s)`
not negative, and `s` outside `[0, 1]` only where `/Extend` says so — falling back to the
lesser root when the greater is not, and `None` where no blend circle passes through the point
at all. `pdf_render::RadialRaster` evaluates it at each device pixel's centre over the shape's
own device bounds, and each backend draws the result as an image confined to the shape.

**This is `MeshRaster`'s construction one shading type over**, deliberately: the clause states
an algorithm no rasteriser's native primitive implements, so it is evaluated once in the shared
crate and the *colour* is identical on every backend while the *edge* is the shape's,
antialiased as every other fill's is.

**The exact evaluation is paid for only where the clause's rule can change a pixel, and that
condition is derived rather than tuned.** It is the sign of `a`:

- **`a < 0`** — the centres are closer together than the radii differ, so one circle contains
  the other: NOTE 2's sphere. `g(s) = |p − c(s)| − r(s)` is convex in `s` (a norm of an affine
  function minus an affine function) and runs from `+∞` to `−∞`, so it is monotone and has
  exactly one root. The other root of the *squared* equation is where `|p − c(s)| = −r(s)`,
  which is not a circle. There is nothing to choose between, so a conical gradient cannot pick
  the wrong one.
- **`a = 0`** — internally tangent; one root has gone to infinity and the same argument holds.
- **`a > 0`** — NOTE 3's cone. `g` runs to `+∞` in both directions and can cross zero twice, so
  a point can lie on two blend circles and the clause's tie-break has work to do.

So `is_a_cone` is `|Δc|² > Δr²`, and it is the exact condition under which the clause and a
gradient differ — not a threshold anybody chose. Every other radial keeps its native gradient
and the cost of this change is zero on the pages that do not contain a cone: the CPU backend's
share of the quorra corpus gate moved 2.70 s → 2.75 s over 957 pages, which is noise.

**Only a fill takes the exact path.** A stroke whose paint is a cone keeps the gradient in all
three backends, because a stroke's outline is not the shape any of them is handed and no corpus
document strokes one. That is a stated departure rather than an oversight, and the three
backends make it identically, so the cross-backend gate still means what it says.

## Consequences

`radial_gradients.pdf` pages 4 and 5, at the page's own scale:

```text
                   ink        worst tile   worst mean   ssim
page 4  before    66.05         108.49         8.62     0.8570
page 4  after     70.62          15.78         4.42     0.8768
page 5  before    67.19         109.58         8.67     0.8531
page 5  after     71.42          15.78         4.52     0.8743
```

Worst tile went from 2.7× the bound to well inside it. Both pages stay `ambiguous`, and
`AMBIGUOUS_RADIAL_CONE` now records why that is the *right* verdict rather than a departure:
twenty-four antialiased boundaries and twenty-four ramps in front of five rasterisers is
§10.7.4's subject, not §8.7.4.5.4's. **A group that said "we are wrong" for twenty-six sessions
now says the opposite, which is the argument for letting a group say the first thing.**

§8.7.4.5.4's ledger row is `implemented`, from `partial`.

`test_scenes::radial_cone` is the shared fixture, used by `render-gpu`'s and `render-quorra`'s
headless suites; the CPU side of the first also asserts the clause's own colour at the point
the arithmetic was done for — 133, 0, 122 at `s = 5775/12075 = 0.478261` — because agreement
about a blank page would be agreement about nothing. Checked by switching quorra's cone path
off: mean error 36.03 against a bound of 0.5.

### What is given up, and to what

The shading's own boundary is point-sampled and therefore not antialiased, which is exactly
what `MeshRaster` gives up and for exactly the same reason. It shows only where a shading ends
*inside* the shape it is painting, and §10.7.4 asks for a hard edge in any case — this tree's
departure is in the other direction and is ADR 0025's.

### Where the todo's design was wrong, and it is worth recording

`doc/todo/12` said the evaluator "needs the target's extent, which is why this belongs beside
the rasteriser", reading NOTE 1's "the blend circles continue as far as that s value for which
r(s) is large enough to encompass the shading's entire bounding box". It does not. That
sentence describes where painting stops being *visible*; solving for the root answers "which
circle passes through this point" directly, and no bounding box enters it. The extent is needed
only to decide how many pixels to evaluate, which is a cost question and not a correctness one.
