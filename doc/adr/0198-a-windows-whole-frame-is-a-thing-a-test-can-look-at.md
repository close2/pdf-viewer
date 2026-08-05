# ADR 0198 — A window's whole frame is a thing a test can look at

Status: accepted, 2026-08-06 (sessions 339 and 340).

## Context

The project owner reported two things breaking at high magnification: the page's characters going
wrong on the way up and staying wrong on the way down (`extensive` reading `extens:ve`), and the
sidebar disappearing above about 2000%, leaving its background rectangle shifted down the screen.
`doc/todo/12` carried both and believed them unrelated. The first was reproduced offscreen in the
three-hundred-and-thirty-seventh session and reported as `doc/QUORRA_FEEDBACK.md` §11. The second
had **no instrument at all**, and the reason is worth stating exactly:

> Every gate in this tree rasterises **one** display list.

The corpus gate and the oracle rasterise a page. `render-quorra/tests/corpus.rs` rasterises a page
at 1×, 2× and 4×. `viewer-ui/tests/panel.rs` rasterises the panel alone. `zoom_ladder` magnifies a
page and draws no chrome. A **window** draws several lists into one scene — the page under its
target transform, the overlays at identity over it — and that combination was what broke.

## Decision

### The scene a window presents is built in one place, and it can be read back

`present::build` was a method on `QuorraPresenter`; it is a free function now, and
`QuorraRasterizer::rasterize_frame` calls it with `Target::Readback` where `present` calls it with
`Target::Surface`. One assembly, two destinations. A second copy would have been two scenes that
drift, and the whole value of the instrument is that it draws *what the window draws*.

The medium is the bottom of that scene rather than imposed afterwards, which is how the surface
path already worked and is right for both: a window has an opaque background.

### The closed form: chrome may not depend on the page's magnification

**The overlay is the same display list at the same target on every rung of a zoom ladder, so its
pixels may not change when only the page's magnification does.** No reference renderer is asked
what a sidebar should look like; the frame is compared with a frame of this test's own. That is
what makes it a gate rather than a comparison, and it is the kind of statement `CLAUDE.md`
principle 5 prefers — a property, derived from what the host does, rather than a golden image.

Two qualifications, both measured rather than assumed:

- **One reference per coverage lane.** `viewer-ui` switches quorra's lane above 10×
  (`GPU_COVERAGE_MAGNIFICATION`) and the two lanes are two rasterisers, so a rung that switches
  lanes differs for a reason that is not a defect — 0.42 mean over this panel, glyph
  antialiasing. Within a lane the frames must agree.
- **The tolerance is 0.01 and the residual under it is 0.0003.** A device with no history at all
  still differs from the rung before it by that much, one glyph edge at worst 16 of 255, because
  the panel's own text is composited over a page drawn at another scale. What the gate was written
  for is 3.77 — four orders of magnitude above the noise, which is what a threshold should look
  like.

### The control that made two defects one

A device per rung, beside one device drawing the rungs in order. **A device with no history was
clean at every rung; one device drawing them in order lost the panel's rows at 3000% and 4600% and
got them back at 6400%.** So it was state carried between frames rather than anything about the
magnification — the same control §11 had run on the page's own glyphs, with the same answer and
the same non-monotone shape. That is what said the sidebar and the wrong glyph were one defect,
and upstream's diagnosis confirmed it: the winding texture is kept between frames and grown to the
tallest sheet any frame has needed, clip space spans the attachment while `vs_winding` divides by
the sheet, and a shorter frame's geometry is stretched by `held ÷ sheet`.

**It also refuted the hypothesis `doc/todo/12` had carried** — that the page's target crossing
16384 truncated the commands encoded after it. The page's target never reaches quorra at that size:
`present` passes the *window's* width and height with the magnification in the transform, and the
two wrong rungs were smaller than the clean one above them.

## Consequences

- **Both halves are closed.** quorra `52b07f29` is pinned; every rung of `zoom_ladder`'s descent
  equals its ascent, `chrome_ladder`'s one-device pass equals its device-per-rung pass, and §0's
  corpus gate is unmoved at 913 / 43 / 1 / 17 on this machine's real adapter. In a real window
  under `Xvfb` the panel's ink is flat at 19.82 to 19.89 from the fitted view to the 6400% clamp
  and back. `doc/todo/12` is deleted.
- **`viewer-ui` keeps switching coverage lanes at 10×.** The mitigation `doc/todo/12` held in
  reserve — stop switching — would have cost the ten-fold frame time the switch was measured to
  buy, and there is now nothing to mitigate.
- **The gate is checked by breaking it**, which is this tree's rule for a new gate: pinning
  `0a1ffb13` back for one run fails it at 3000% with `mean 3.7733`, naming the example that writes
  the pictures.
- **One fact about hosts came out of building it.** The ladder could not pass 20× until it answered
  `Command::RenderReady { rendered: Rendered::Presented }`. That answer is how a host tells
  `viewer-core` it is tier 2 — it draws its own frames at its own size and holds no whole-page
  raster — and until it does, `MAX_PIXELS` bounds a raster nobody was going to allocate and the
  core refuses the page by name. Correct on both sides, and worth knowing: **a tier-2 host that
  never says `Presented` cannot zoom past about 20×.**
- **`o` toggles the sidebar and the sidebar starts open on this document**, which cost a wrong
  reading before it was checked: `/PageMode` opens the panel, so the first `o` in a scripted run
  *closes* it. Three presses and an ink measurement settled it. Trap 1, in the window.
