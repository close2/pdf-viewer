# ADR 0556 — Two answers a dependency asked for, and one enum it did not name

Status: accepted, 2026-08-23. Session 699. Adds
`crates/render-quorra/examples/sampled_lane_column.rs`, amends §10.7.4's ledger row, and answers
`doc/QUORRA_CLIP_LANE_AND_UPLOAD.md` §6's third question and its closing paragraph. No pixels move.

## 1. The sampled lane's routing: leave it alone, and their remedy would not have reached it

quorra's ADR 0076 records the sampled coverage lane's zero as a **non-conformance** of ISO 32000-2
§10.7.4 rather than as a tolerance, and this tree agrees with that reading — the clause's sentence
is unconditional:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is.

and its NOTE 1 puts the pixel a *boundary* crosses inside the requirement. A lane that leaves such
a pixel at zero does not meet it, under the binary reading or the anti-aliased one. §10.7.4's
ledger row carries it as this tree's departure (4), because a backend of ours has it.

They offered to divert marks whose width is not a multiple of the pitch to the processor, at a cost
in exactly the population that chose the lane for speed, and asked for our corpus column first.
Here it is. `examples/sampled_lane_column` walks each of the corpus's 957 first pages at the
magnification `viewer-ui` actually takes this lane at — **ten**, `GPU_COVERAGE_MAGNIFICATION`,
because below it no frame this program draws uses the lane at all — and counts marks by the narrow
side of their device bounding box, skipping anything under the pitch since their ADR 0070 already
keeps those on the processor:

| | at 1× | at 4× | at 10× (shipped) |
|---|---:|---:|---:|
| marks on the sampled lane | 287 475 | 313 303 | 326 269 |
| would divert | 262 747 | 283 921 | **288 129** |
| share of the lane's marks | 91.40 % | 90.62 % | **88.31 %** |
| pages with at least one | 681 of 958 | 680 of 957 | 679 of 957 |

At the shipped magnification the diverted marks fall by width band —
`[< 1 px, 1–4 px, 4–16 px, > 16 px]` — as **[20 290, 26 844, 55 355, 185 640]**. Sixty-four per
cent of them are wider than sixteen device pixels, where the lattice is finer than one level of an
eight-bit raster; seven per cent are the band under one pixel where the relative error `p/w` is a
quarter or worse.

**So the answer is no, and the reason is not only the price.** Diverting on width would move nine
marks in ten off a lane whose entire purpose is that a zoom gesture costs what standing still costs
— this tree measured 0.44 ms a frame at 8× against 4.4 ms at 12× when it chose the crossover — and
it would **not remove the non-conformance**, because the zero is a property of the lattice rather
than of the width. Their own §3 says as much: "the error here is `p·k − w` … independent of how
wide the mark is", and "no threshold change reaches it". A remedy that costs 88 % of a lane and
leaves the clause where it was is the wrong trade in both directions.

What is asked for instead is the smaller thing they have already done: the bound stated on
`Coverage::Gpu`'s own rustdoc where the lane is chosen, so that a caller who needs exact area asks
for `Coverage::Cpu`. This tree's host will keep choosing the sampled lane above ten times
magnification, where a hairline is 2.5 % wrong and a rule of any consequence is less.

## 2. `#[non_exhaustive]`: yes for the three error enums, and **no** for the one they did not name

They observed that every `RenderError` and `SceneError` addition they have called additive was
additive by our luck rather than by contract, that we hold no exhaustive match over either, and
that marking both is "yours to time, not ours to spring".

**The first half is verified rather than taken.** Every mention of either enum in `crates/`,
`tools/` and `fuzz/` is one of four things: two `#[from]` conversions in
`render_quorra::QuorraRasterError`, one `Err(RenderError::SurfaceUnavailable { .. })` arm with a
catch-all `Err(problem)` beside it in `viewer-ui`'s surface loop, two `matches!` in
`render-quorra/tests/headless_quorra.rs`, and a return type. Not one exhaustive match. The break
costs this tree **nothing**: no source edit, no behaviour change, no test.

So the recommendation is **accept, and take it now rather than time it** — for `RenderError`,
`SceneError` and `DeviceError` alike, since we hold no match over the third either. A break that
costs nothing does not get cheaper by waiting, and every release in between is one where "additive"
is a claim about our source rather than about their type.

**And there is one enum in that family we would ask them not to mark.** `viewer-ui`'s `swapchain()`
holds a genuinely exhaustive match over `quorra_gpu::SurfaceProblem` — five arms, no catch-all —
and that is deliberate on both sides. Their own module comment calls it "the one enum here whose
completeness is not ours to argue: its five variants are exactly the five non-success arms of
`wgpu` 30's `CurrentSurfaceTexture`". An enum that mirrors somebody else's closed set is one where
an exhaustive match is the *feature*: when `wgpu` grows an arm, the compiler is what tells this
tree that a window state has appeared that its event loop has no policy for. Marking it
`#[non_exhaustive]` would replace that with a silent catch-all, which is `CLAUDE.md` principle 1's
error swallowed in the one place a host must not swallow one.

The general rule the pair makes, and it is worth more than either: **mark the enum whose variants
you own and expect to grow; leave open the enum that is a faithful mapping of a closed set you do
not own.** `pdf_render::Command` and `pdf_render::Paint` are already `#[non_exhaustive]` in this
tree for the first reason, and `element_alpha_is_shape`'s `_ => false` is what that buys.
