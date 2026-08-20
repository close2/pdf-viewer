# 607 — The column the third host could not hold

The second round of the block on the owner's decision that **the UI is now work**, and the one that
finishes what 606 deliberately left: `viewer-ui` draws Table 29's arrangement now, so all three hosts
are level again.

Date: 2026-08-20.
ADR: [0442](../adr/0442-the-column-the-third-host-could-not-hold.md).

Touched: `crates/render-quorra/src/present.rs` and its test, two examples and `zoom_frame`;
`crates/viewer-host/src/{arrangement.rs (new), lib.rs}`; `crates/viewer-gtk/src/host.rs` and
`crates/viewer-qt/src/host.rs` (each lost its copy of `next_layout`);
`crates/viewer-ui/src/software.rs`, `crates/viewer-ui/src/bin/pdf-viewer.rs` and
`bin/pdf-viewer/{app.rs, dispatch.rs, presentation.rs, renderer.rs, stale.rs, surface.rs, timing.rs,
window.rs}`, two of its tests and one example; `doc/conformance/ledger.toml` (§7.7.2),
`doc/ui-boundary.md`, `doc/todo/30`, `doc/todo/37`, the ADR and this file.

The spec-driven half is in the same commit and is its own thing: **§8.6.5.9's black point
compensation**, in `pdf-model`, with a new `crates/pdf-model/tests/rendering_intent.rs` and four
ledger rows (§8.6.5.9, §8.6.5.8, §8.9.5.1, §11.7.5.3) — see below.

## The route, and the argument

606 named two and chose neither. This round took the **second** — the frame carries several placed
lists — and the argument is in the ADR. Four things decided it, and the first is the strongest
because it is the specification's:

- **A merged display list cannot hold two page groups.** §11.4.7 makes the blending colour space a
  property of *the page*, and `DisplayList` carries it — with a whole companion list for the black
  component — once per list.
- `Command::Group` carries **no transform**, so placing a page inside a merged list means rewriting
  every command's, every clip's and every soft mask's, per magnification.
- A merged list is a **new allocation**, and the address is the identity `render-quorra`'s retained
  scene and `crate::cache`'s pinned resources are keyed on (ADR 0351).
- **Trap 2 does not point at the merge.** Where the pages go is not a decision a backend makes:
  `viewer_core::layout` made it and states it as a `TargetSpec` per page. A merge would need those
  placements handed to it and would then hold a *second* statement of the arrangement.

`PresentFrame::page` became `PresentFrame::pages`, a slice — which is what `overlays` already was —
and `build`'s `if let` became the loop next to it.

## It needed no message, which is the part worth keeping

A tier-2 host hands back no pixels, so `Query::Frame` answers `Answer::None` for it. The obvious move
was a new query; it is not needed. **`Query::PageGeometry` answers `Answer::None` for a page the
arrangement does not show**, which is the whole of what a host holding one `RenderRequest` per page
needs in order to know which have scrolled off. `App::arrangement` is those six lines.

## Reprojection across a column

`Stale::settled` holds a *list* of pages now, and `one_placement` composes each page's own
`settled⁻¹ ∘ asked` and answers only where they all agree.

- **A scroll agrees exactly** — every page moves by the same distance — so a column reprojects as a
  single page always did. Seen on the screen: `approximated` on every wheel notch, the real frame one
  tick later.
- **A zoom does not**, and a sixth refusal says so: `Refusal::Rearranged`. `viewer_core::layout`'s gap
  is stated in logical pixels and does not scale with the magnification while the pages do, so no one
  affine carries both. Exact rather than tolerant, because a tolerance here is a number nobody
  measured a purpose for — the mistake `doc/todo/37` already records twice.

## What was found on the way, and was not on any list

**§11.4.7's 𝑊 is the page's, and what a window shows where there is no page is not the standard's
subject at all.** Found by making the medium grey to see the gap between pages, and watching the
*pages* turn grey: a page paints no white of its own, so the medium behind the whole window is
serving as §11.4.7's backdrop —

> The page group shall be treated as an isolated group, whose results shall then be composited with
> a backdrop colour appropriate for the medium. The backdrop is nominally white

— and Table 141 names it "[i]nitial colour of the page". One colour is doing two jobs, in
`render-quorra`'s `build` and in `render-cpu`'s `impose_on_medium`, and it was invisible for four
hundred sessions because one page filled the window. **The colour was put back to white and the
finding written down** rather than half-fixed at the end of a round that had already changed the
frame's shape; `doc/todo/30` carries it with the clause under it. The two native hosts never had it —
their surround is the toolkit's own window background.

**`next_layout` was about to be written a third time.** `viewer-gtk` and `viewer-qt` each wrote it in
606, one with a comment saying the other had it "deliberately". It is `viewer_host::arrangement` now,
which is that crate's own test for what belongs in it: which key cycles is a toolkit's, what the next
arrangement is is not.

## The measurements

Time to first present, `--trace=launch` under Xvfb :78 at 900×1100, three runs apiece, both documents
in the `OneColumn` each of them states:

| | pages | `viewer-ui` | `viewer-gtk` |
|---|---|---|---|
| `doc/PDF20_AN001-BPC.pdf` | 5 | 120.5, 120.0, 121.1 ms | 123.6, 119.9, 109.5 ms |
| `doc/ISO_32000-2_sponsored_EC3.pdf` | 1023 | 120.8, 135.9, 134.7 ms | 163.7, 160.2, 163.2 ms |

A document two hundred times longer opens in about the same time on the tier-2 host, on the path that
now interprets several pages rather than one. `viewer-gtk`'s numbers are 606's within the noise, which
is the control.

## What was seen on the screen

Xvfb :78, 900×1100, `viewer-ui`.

- `PDF20_AN001-BPC.pdf` opens in a continuous column because that is what the file asks for, the
  panel opens because `/PageMode` is `UseOutlines`, and the trace says `frame p1+1 543cmd` — two
  pages, both interpreted.
- `l` cycles all six and each says which it moved to. `TwoColumnLeft` shows 1|2 over 3|4;
  `TwoColumnRight` leaves page one alone on the right and puts 2|3 in the row below, which is the
  bound book the clause describes; `TwoPageLeft` and `TwoPageRight` show one spread, centred, with
  nothing scrolling past it.
- The wheel crosses two page boundaries: fifty `xdotool click`s — a hundred events, because a notch
  is two here — take the title bar to *2 — page 3 of 5 — References*, which is §12.4.2's label beside
  the index and §12.3.3's section following the scroll, over a screen holding two pages
  (`frame p3+1 4059cmd`).
- **Fifty-one reprojections in that run**: every wheel notch prints `approximated … the real frame is
  being drawn` and the rendering lands on the tick after it. That is the property this round had to
  keep and the reason `Stale` composes per page rather than per frame.

**And the gap between neighbouring pages is invisible**, which is the §11.4.7 finding above and the
one thing on the screen this round did not fix.

## The spec-driven half — §8.6.5.9, and it was none of the six refusal shapes

The row said the requirement was implemented and the row beside it (§8.9.5.1) declined the rest on a
*corpus count*, which is `CLAUDE.md`'s two-denominators failure rather than one of `doc/habits.md`'s
six shapes: a coverage requirement answered with a robustness measurement. The clause:

> If the current render intent of an object is AbsColorimetric then the value of UseBlackPtComp shall
> be treated as OFF .

The load-bearing word is **object**, and it was implemented as an assignment: the intent and
`/UseBlackPtComp` shared one graphics-state field, so `ri /Perceptual` after `/UseBlackPtComp /OFF`
silently turned compensation back **on**, and `/UseBlackPtComp /ON` after `ri /AbsoluteColorimetric`
compensated although the intent in force says it shall not. Beyond that: §8.6.5.8 states **three**
routes to an intent and the row named two, with Table 87's `/Intent` unread anywhere in the tree; and
the two it did name reached a path's colour and a glyph's and nothing else, because `image.rs`,
`shading.rs` and `mesh.rs` each passed a literal `true` at every `Compositing::paint` call.

`GraphicsState` splits into `intent` and `use_black_pt_comp`, combined **per object**; a new
`colour::Conversion` pairs the flag with the target that was already threaded everywhere, so an
omitted black point is a compile error rather than a habit; `RasterCache`'s key and `shading::Cache`'s
carry it, because one page can draw one stream under two intents. Eight tests in a new
`rendering_intent.rs`, on a hand-assembled `lut16Type` profile whose darkest colour is a tenth of its
white point, plus one in `image_reuse.rs`. Four ledger rows updated, none of them changing status —
each still owes something real.
