# A frame every refresh — 60 Hz as the floor, 120 Hz as the target

Status: **asked for by the project owner**, as the second half of the reprojection they asked for
reluctantly in [`37`](37-a-frame-that-says-it-is-stale.md). Their words: *"I want it to be able to
render every frame (at least 60Hz but 120Hz should be the target). … We should still try to render
a correct image every frame, but if we miss, we should interpolate from the last frame (even if the
last frame was already incorrect). If possible and a frame is delayed we could use the delayed
frame for further interpolated frames."* Nothing is built.
Priority: 36 — capability, and the first item in this tree whose acceptance is a *rate*
Witness: `tmp/Entwurf.pdf` — **not in the repository**, so no test may name that path. Its zoom step
is 275–420 ms with encode threads on (ADR 0377), against 8.3 ms at 120 Hz: **the correct frame will
be missed by a factor of thirty to fifty, and that is the case this item is for.**
Instrument: the window's `--trace` frame line and its summary, which already count `approximated`
frames (ADR 0378); what they do not yet report is a *cadence*.
Clauses: none — presentation. §10.7.4 does not reach it: nothing reprojected is a rendering.
Code: `crates/viewer-ui/src/bin/pdf-viewer/{stale,surface,window}.rs`, `crates/render-quorra`

## What changes from [`37`](37-a-frame-that-says-it-is-stale.md)

That item reprojects **once**, on a view change, and waits. This one presents **on the display's
own cadence**, whatever the renderer is doing:

- **The loop no longer waits for a frame.** Today a view change requests a render and the window
  redraws when it lands. At 120 Hz the window must present every 8.3 ms whether or not anything
  landed, which makes the presenter a *clock* rather than a *reaction* — the largest structural
  change this item carries, and the one to design first.
- **A reprojection may follow a reprojection.** The owner allows it explicitly. But *how* it does
  matters: resampling an already-resampled image compounds the blur, so the base should stay the
  **last real frame's raster** with the transforms **composed** — one resample from the true
  pixels, never a chain of them. Where that is impossible (the real frame's raster is gone), say
  so and state what the chain costs.
- **A late frame is still useful.** When a delayed frame finally arrives it becomes the new base
  even if the view has moved on — its pixels are truer than the ones being reprojected, and the
  composed transform simply changes.

## What is not settled, and must be before anything is built

1. **Where the pixels come from at 8.3 ms.** ADR 0378's `capture_presented` costs **19–36 ms** —
   more than the whole frame budget — because it reads back. A cadence at 120 Hz needs the
   transform applied to a texture quorra **already owns**, without a round trip. This is very
   likely an **ask to quorra** (the shape: present the retained frame's target under an affine),
   and `doc/QUORRA_FUNCTION_PAINT.md` is the model for writing one. Session 547 was asked to find
   out what exists; whatever it reports is the starting point.
2. **What "every frame" means when nothing changed.** A still window must not spend a GPU frame
   redrawing identical pixels 120 times a second — the retained frame already replays at 21–35 ms,
   and a still window should present nothing new at all. The rate is a *ceiling on latency*, not a
   duty to burn power, and the item is wrong if it makes an idle viewer hot.
3. **Where the cadence comes from.** The display's refresh rate is a property of the surface, not
   a constant; `winit` reports it, and a target that is not a multiple of it is a stutter machine.
4. **What the gates see.** [`37`](37-a-frame-that-says-it-is-stale.md)'s rule 2 is unchanged and
   absolute: **nothing that judges a picture may ever see a reprojection**, and a presenter that
   runs on a clock must not make that harder to guarantee. It is structural today (a private
   module of a binary) and must stay structural.

## The five rules still bind

All five of [`37`](37-a-frame-that-says-it-is-stale.md)'s, unchanged, plus one this item adds:

6. **The cadence is measured, not asserted.** A round claiming 60 or 120 Hz reports the
   distribution of intervals between presents on the witness — not a mean, since the failure mode
   is a long tail — and says what fraction of frames were correct rather than reprojected. "We
   present every 8.3 ms" and "we show the right pixels" are two different claims and both belong
   in the record.

## Why the owner's framing is right, and worth keeping

*"We should still try to render a correct image every frame"* — the reprojection is a floor under
the experience, never a substitute for the frame being fast. Everything that makes a frame cheaper
(ADR 0377's threads, ADR 0374's raster cache, ADR 0351's retained frame) reduces how often this
item is visible, and the standing intent from [`37`](37-a-frame-that-says-it-is-stale.md) holds:
**the better the renderer gets, the less this should ever be seen.**
