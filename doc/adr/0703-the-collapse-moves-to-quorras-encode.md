# 0703 — The collapse moves to quorra's encode

**Status.** Accepted — the second §4.5 amendment, symmetric with ADR 0701's stroke
width and driven by ADR 0702's measured finding: the worst page's scene could never
survive a zoom because ~20 of its 58 009 fills are hairline rulings whose §10.7.4
marks this crate split out *before* the scene, sized and floored on one placement's
pixel grid — identically at 0.55× and at 2.4×, so no zoom outruns them. Quorra's side
is their ADR 0086.

## What changed, on this side

`render-quorra` stops calling `pdf_render::split_collapsed_fill` at the quorra
boundary. The whole original path crosses — which also keeps its `Arc` cache
identity — and quorra's upload finds the collapsed subpaths once (a property of the
control points), its encode placing each §10.7.4 mark per viewport: the floored
device-pixel run under an axis-preserving placement, the one-device-pixel band under
a rotation or shear, mirrored statement for statement from `pdf_render::collapsed`.
The collapsed fill's `consume_view` mark is deleted with the split, so a page whose
only view-readers were its rulings is **view-free** and its scene survives every
zoom, scroll and resize (ADR 0702).

`render-cpu` and `render-gpu` are untouched: they still split through `pdf-render`,
which remains the cited implementation and the oracle's path — the same containment
as ADR 0701, so the cross-backend gates compare the two implementations continuously.

## Measured

The phase probe (1200×900, alternating placements so every frame is a zoom step,
RADV 890M, `Coverage::Compute`), the worst page (`doc/todo/44`):

| | scene | handover | whole step (headless) |
|---|---:|---:|---:|
| before (ADR 0702) | 8–15 ms | rebuilt | ~70–123 ms |
| after | **0.0** | **0.0** | ~69–110 ms |

The scene is built once and reused for every subsequent zoom step; what remains of
the step is quorra's own encode (15–26 ms), upload (18–28 ms, the count stall inside
it) and device time — the terms quorra's ADR 0084 names for its stages three and
one, none of them this crate's.

## Held by

Quorra's suite (620) including their `tests/collapsed_fills.rs` — one scene, three
viewports, three floored rows — and this side's sixty `render-quorra` tests and the
full workspace, unmodified: the oracle comparison holds because quorra's mark
arithmetic mirrors the split's, within ulps the relaxed contract (quorra's ADR 0082)
covers. The commit of this side waits on the quorra pin reaching ADR 0086, exactly
as the lock discipline requires.
