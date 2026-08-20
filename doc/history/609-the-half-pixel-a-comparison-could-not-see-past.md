# 609 — The half pixel a comparison could not see past

The fourth round of the block on the owner's decision that **the UI is now work**, and the one that
takes the two defects the column left behind: a comparison that refused ordinary scrolls, and a
drag that stopped at a page boundary.

Date: 2026-08-20.
ADR: [0444](../adr/0444-the-half-pixel-a-comparison-could-not-see-past.md).

Touched: `crates/viewer-ui/src/bin/pdf-viewer/{stale.rs, surface.rs, typing.rs}`;
`crates/viewer-core/src/{open.rs, viewer.rs, query.rs}` and `tests/{headless.rs,
selection_census.rs}`; `crates/viewer-confined/src/protocol.rs`; `crates/viewer-ffi/src/session.rs`
and `include/pdf_viewer.h`; `doc/conformance/ledger.toml` (§9.4.1, §12.5.2, §12.5.6.10, §14.8.2.5),
`doc/ui-boundary.md`, `doc/todo/30`, `doc/todo/37`, the ADR and this file.

## Item 1 — the bound, and the order it was arrived at

`stale::AGREEMENT = 0.5` device pixels, applied to the largest distance the two placements put a
texel of the picture at, over the four corners of it. Both halves are the geometry rather than a
tolerance: the difference of two affines is convex, so a convex polygon's maximum is at a vertex
and four points answer exactly; and half a pixel is the largest distance a point can move without
leaving the pixel it is a sample of, which is the argument `TargetSpec::for_page` already rounds by.

**The measurement was taken after the derivation, and what it found is a separation.** Fifteen
scrolls of a real column disagree by 0 to 0.000183 px — twelve of them exactly zero, three of them
the residual that `==` refused — and five zoom steps by 1.25 to 2.75 px. Four orders of magnitude,
with the bound between them touching neither. On the screen, under `Xvfb` on the specification in a
four-page column: scrolls carried at 0.0000–0.0001 px, Ctrl + wheel zooms refused at 1.2501 to
5.3749 px, the layout change refused at 877.9067 px.

The trace prints both populations now — the disagreement absorbed on the frame line, the one
refused in the refusal — so the next round to touch this re-measures without instrumenting
anything. `stale.rs`'s test builds its column out of `viewer_core::layout` rather than out of its
own arithmetic, which is why it can see the residual at all: the file's own `column` helper stacks
exact translations and nothing in it ever divides.

## Item 2 — a selection is two `(page, offset)` ends

`Chosen` is `{ from: Spot, to: Spot }`, the drag no longer refuses to leave the page it started on,
and `Answer::Selected`'s `text` is a `Cow` — borrowed for a selection inside one page, assembled
only for one that crosses a boundary. **A variant changing shape rather than a message added**, on
596's and 606's precedent: the host already had exactly half the answer.

The spec-driven half is the same work rather than beside it. §9.4.1's "shall not persist from one
text object to the next" is why there is no document-wide offset and why a *pair* is the answer;
§12.5.2's "A given annotation dictionary shall be referenced from the Annots array of only one
page" is why §12.5.6.10's mark-up over such a selection is one annotation per page and cannot be
one shared; §14.8.2.5 states no order between two pages, so the logical selection joins per page and
its refusal reaches further. Four ledger rows say so; `spec-errata emit` was run over the
specification first, and the §12.5.6.10 erratum it prints is about `/Path`, not about any of this.

Two things that did not change, both on purpose: `Selection::All` is still one page, because it is
what the census and `pdf-retrieve` rest on byte for byte; and a selection now dies when **any** page
it covers leaves the screen, which is the conjunction rather than the disjunction, because text with
a hole in it is worse than no text.

## The hosts

All three have item 2 and needed no line about pages: each asks `Query::Selection` per repaint and
draws the quadrilaterals it is handed, in the viewport's own device pixels. One host has item 1, and
that is structural rather than a deferral — the reprojection exists only in the tier-2 host, because
tier 1 is handed a raster per page. `doc/todo/37` carries it as it did before.

## What running it said

`viewer-ui` under `Xvfb`, four pages in a column: a drag down the window selected across two page
boundaries, the quadrilateral count growing from 176 to 231 as it crossed them, and the wash on the
screen covering about 400 rows where one page of that arrangement is 244.

## What is left

`doc/todo/30`'s second column item — `Query::Reports`, `Query::Readback` and
`Query::AccessibilityTree` answer for the current page alone — and `doc/todo/37`'s two: the
processor's window, and the identity that would let a `SinglePage` page turn use a retained page.
