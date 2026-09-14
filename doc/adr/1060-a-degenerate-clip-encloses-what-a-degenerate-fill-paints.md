# ADR 1060 — A degenerate clip encloses what a degenerate fill paints, and that is one decision

## Status

Accepted, 2026-09-14. Session 1045. Decides what session 1042 recorded on §8.5.3.3.1's ledger
row and did not take.

## Context

ISO 32000-2 §8.5.3.3.1: "If a subpath is degenerate (consists entirely of one or more points at
the same coordinates), the subpath shall be considered to enclose the single device pixel lying
under that point; the result is device-dependent and not generally useful." This tree does not
paint that pixel for a fill. The departure is decided once, in `pdf-render/src/collapsed.rs`, in
the crate every backend reads, and `a_single_point_subpath_is_not_this_rule` pins it; ADR 0154
argues why the zero-*height* rectangle two sentences over is painted while the point is not — the
clause hedges its own answer for the point and attaches no hedge to a shape with a direction.

Session 1042 found, on `issue6413.pdf`, a form whose `/BBox` is `[0 0 0 0]`. Its clip is a
degenerate subpath, and §10.7.4 says what a clip is: "For clipping, the clipping region consists
of the set of pixels that would be included by a fill operation." Read against §8.5.3.3.1 alone,
the clip owes one pixel; the row recorded that as a debt priced at one device pixel.

## Decision

**The clip follows the fill, because §10.7.4 defines it by the fill.** What a degenerate clip
encloses is whatever a degenerate fill paints on this device, and this device paints nothing for
it, by the decision above. A clip that admitted the pixel the fill declines would make the two
halves of §10.7.4's own definition disagree with each other: the region a `W n` admits would not
be "the set of pixels that would be included by a fill operation" of the same path.

So there is one departure, not two, and it lives where it always did — `collapsed.rs` — rather
than in a backend's clip construction. Reversing it means reversing both at once, in the shared
crate, and nothing about the clip is decided anywhere else.

Measured before deciding, on `render-cpu` at one pixel per unit: `10.5 10.5 0 0 re W n` over a
full-page fill, `10.5 10.5 0 0 re f` and `10.5 10.5 m h f` each mark no pixel. On `issue6413.pdf`
the one pixel's centre lies outside the image at 72 dpi in any case, and all four references
draw the same blank there.

## Consequences

- §8.5.3.3.1's row stays `partial` with one recorded departure covering the fill and, through
  §10.7.4, the clip. It is not re-opened for a clip unless it is re-opened for the fill.
- **The neighbour that is a defect is not covered by this.** A clip collapsed along *one* axis —
  `5 20.5 30 0 re W n` — admits nothing, while the fill of the same rectangle paints the
  thirty-pixel row `collapsed.rs` builds, and §10.7.4's own EXAMPLE is "[a] zero-width or
  zero-height rectangle paints a line 1 pixel wide". `split_collapsed_fill` is asked by each
  backend's fill and by neither's clip. That is owed, in the backends, and §10.7.4's row says so.
