# 0378 — A frame that says it is stale, and the five rules that make a wrong picture defensible

**Status.** Accepted. Session 543. Builds `doc/todo/37`'s device half; amends that file down to the
one path it does not cover. Rests on ADR 0351 (the retained frame), ADR 0368 (what a zoom step
costs and why) and ADR 0297 (the precedent for keeping a per-frame answer in the window's backend).

## Context

ADR 0368 measured a zoom step on `tmp/Entwurf.pdf` — one page, 58 009 display commands — and found
a **640 ms** frame of which 74% is quorra's encode, none of which survives a change of
magnification by any design either tree can afford. Until this session, what the window did during
those 640 ms was show the *previous* view's pixels, unmoved, and say nothing.

The project owner asked for the fix **reluctantly**, and the reluctance is part of the
specification:

> even though I was hoping we could avoid it completely … When we are fast enough, we can print
> every frame correctly. But when not, we currently don't give any feedback.

The hazard is `CLAUDE.md`'s first principle. A reprojected frame is a *wrong picture, drawn
deliberately*: a raster scaled up is blurred, a scroll reveals an edge the old raster has no pixels
for, and anything the new view would draw that the old one did not is simply absent. Trap 5's whole
point is that a viewer must not quietly show a wrong picture. So this is defensible only under
`doc/todo/37`'s five rules, and **each of them is enforced by a test, an assertion or a type**
rather than by a comment.

## Decision

**Build the reprojection in the presenter, out of pixels the device has already drawn.**

One frame's worth of it, in order:

1. A view change arrives. `App::present` composes the new placement and asks
   `crate::stale::Stale::plan` whether the pixels on the screen are *this page* under another
   view, and whether the last real frame was slow enough for the wait to be worth answering.
2. If it is, `QuorraPresenter::capture_presented` draws the scene `FrameSlot` is **already
   holding** once more, into `Target::Readback`, at the same viewport and coverage lane. quorra's
   `EncodeKey` covers the viewport and deliberately not the target, so this **replays** the encode
   rather than making one — `Captured::replayed` is the observable that says so per call.
3. The host builds a display list of exactly one `Command::Image`: the captured window, placed by
   `new ∘ old⁻¹` — the affine that carries the old view's device pixels onto the new view's — under
   an identity target transform, which is the space this host's chrome already draws in.
4. That frame is presented, with the *current* chrome over it. The window has answered the input,
   typically within 23 to 51 ms of it.
5. The real frame is asked for in the same expression that records the reprojection, and it
   replaces it.

### Why the pixels come from a readback, when a readback sounds expensive

Four cheaper-sounding routes were checked against the two libraries' own code, not argued:

- **Re-render the retained scene under a changed viewport affine.** `quorra_gpu`'s `EncodeKey`
  holds the viewport's six affine coefficients by their bit patterns, so this is a full re-encode:
  the 640 ms itself.
- **Draw the swapchain texture again as an image.** quorra acquires and presents the surface
  texture internally and a host never holds one; `Device::wgpu()` would mean this tree writing its
  own blit pipeline against a texture it cannot reach.
- **An image backed by a device texture.** quorra's resources are uploaded from host bytes
  (`upload_image(&ImageSpec)`); there is no texture-backed image to hand it, so a capture must
  cross to the host whatever else changes.
- **Rasterise the page again on the processor.** That is the cost this exists to hide.

So a readback of the encode that already exists is not the expensive option — it is the only one
whose price is a replay plus a copy, and the measurement below is what says the price is small
enough.

### The five rules, and the thing that enforces each

| rule | enforced by |
|---|---|
| **1. Never the last word** | `Stale::plan` refuses while one is showing; `MustFollow` is `#[must_use]` and its only method asks the window for the frame that replaces it, so the redraw cannot be forgotten; `about_to_wait` refuses to let the loop come to rest with one on the screen; and *every* frame that is not a reprojection clears the flag, including a frame that drew nothing, so the guard cannot spin. Two unit tests. |
| **2. Nothing that judges a picture ever sees one** | **Structural.** Everything that makes an approximate picture is in `crates/viewer-ui/src/bin/pdf-viewer/stale.rs`, a private module of a **binary**: the corpus and oracle gates, `Query::Frame`, the confined worker, `render_at` and the headless harness are compiled without a line of it, and cannot link to it in principle. The one library addition is `QuorraPresenter::capture_presented`, which hands back *the real frame the window is showing* — `render-quorra` has no notion of a reprojection, and `QuorraRasterizer`, the judged offscreen path, has no such method. A test walks every `.rs` outside `viewer-ui/src/bin` and fails if any of them so much as names one. |
| **3. It says so** | The frame line's outcome word is `approximated`, the legend explains it, a line beside it says which frame it stands in for and what it cost, and the summary prints the count — kept in `Stale` rather than in the frame log, so it is exact whether or not anything is tracing. |
| **4. It costs the real frame nothing** | The pixels are a *replay* of an encode that already existed; a capture that re-encoded is reported by name and switches the feature off for the run. Nothing is asked for at all when the last frame made the device repack its atlas (the retained encode is dead) or when no frame has reached the device. And the threshold below is a multiple of what a reprojection *actually cost*, so one that turns out expensive raises the bar it must clear next time rather than being repeated. |
| **5. It does not fire when it is not needed** | `Stale::threshold` = `SHARE` × a measured cost. Measured, not tasted; see below. **Superseded — ADR 0384.** |

### The threshold, and why it is arithmetic on a measurement

> **This section is wrong, and ADR 0384 replaces what it decided.** It is left standing rather than
> rewritten, because the reasoning below is what a reader has to see in order to understand the
> failure: every sentence of it is about how the bar *responds* to a measurement, and not one asks
> where the first measurement comes from. It comes from drawing a reprojection; a reprojection was
> drawn only above the bar; so **the bar gated its own only sample**, and on any machine quicker
> than this software adapter it could never come down. The project owner ran it on a real graphics
> device: fifteen presents, frames of 80 to 438 ms, and not one reprojection. No value of `ASSUMED`
> fixes that — the fault is the direction of the dependency, not the constant. Rule 5 is now the
> cadence's own period, which is a measurement that exists before anything has been drawn; rule 4
> keeps the ratio below, as a separate check, with *unmeasured* permitting rather than refusing.

`doc/todo/37`'s fourth rule says a reprojection that cannot be produced "within a small fraction of
the frame it replaces" is not to be produced at all. **A tenth** is this project's reading of "a
small fraction", and it is the only ratio in the design; everything it multiplies is measured on
the machine that is running:

```text
threshold = 10 × (this run's most expensive reprojection, or ASSUMED until there is one)
```

`ASSUMED` is the top of the measured band rather than its middle, because rule 4 is a bound. A
machine slower than this one raises its own bar within one step; a machine with a real graphics
device lowers it within one step.

**And that last sentence is the one that was never true**: a machine with a real graphics device
took no step at all, because taking one required a frame over 510 ms and its frames were quicker
than that. ADR 0384.

## The measurement

`Xvfb :77` at 900×1100, llvmpipe, the release binary of this tree, four scripted sessions of
`tmp/Entwurf.pdf` driven by `xdotool`. **Seven reprojections:**

| the frame it stood in for | readback | whole reprojection | share | the real frame that followed |
|---:|---:|---:|---:|---:|
| 982.5 ms | 35.9 | **50.6** | 8.4% | 600.0 |
| 951.0 | 28.7 | **41.8** | 7.1% | 589.4 |
| 937.0 | 24.2 | **38.7** | 6.8% | 567.8 |
| 567.8 | 24.5 | **30.8** | 4.9% | 632.8 |
| 1036.4 | 26.1 | **38.9** | 6.8% | 575.8 |
| 575.8 | 25.4 | **29.8** | 6.0% | 492.6 |
| 492.6 | 19.2 | **23.1** | 21% | 108.7 |

**23.1 to 50.6 ms**, of which the readback is 19.2 to 35.9 — and on this adapter the readback
includes drawing the page again, which is why it dominates. `ASSUMED` is therefore 51 ms and the
bootstrap threshold 510 ms.

**The last row is the honest limit of the scheme and is left in the table rather than dropped.**
The decision is a *prediction* from the frame before it, because what the next frame will cost
cannot be known before it is drawn — and ADR 0368's fourth finding is exactly the case where the
prediction is wrong: a magnification quorra has drawn before costs a fraction of a new one. There
the reprojection cost a fifth of the frame it covered instead of a fifteenth. It is a bounded
error, in milliseconds rather than in correctness, and the alternative — predicting from the page's
command count, say — would be a worse estimator of the same unknown.

### Rule 5, seen rather than asserted

The same session, five view changes in order: `+`, `+`, `-`, `Down`, `Down`.

```text
frame p1 58009cmd presented 1036.4 | …                       the launch frame
frame p1     1cmd approximated 38.9 | …                      +
frame p1 58009cmd presented  575.8 | …
frame p1     1cmd approximated 29.9 | …                      +
frame p1 58009cmd presented  492.6 | …
frame p1     1cmd approximated 23.1 | …                      −
frame p1 58009cmd presented  108.7 | …   ← quorra had drawn this magnification before
frame p1 58009cmd presented   22.8 | … replayed              Down: no approximation
frame p1 58009cmd presented   31.4 | … replayed              Down: no approximation
```

The last two are the rule working: after a 108.7 ms frame the threshold (389 ms at that point in
the run) is not met, so the window shows the frame rather than an approximation of the one before
it. On `doc/PDF20_AN001-BPC.pdf`, whose frames are about 43 ms, the same script produces **six
frames and zero reprojections**, and the summary says so.

> **That last sentence was read as evidence and was the defect itself.** Forty-three-millisecond
> frames on a 16.7 ms refresh are three refreshes each — they *are* misses, and every one of them
> should have been stood in for. Zero reprojections was not rule 5 declining; it was rule 5 unable
> to fire. The same document under ADR 0384's trigger produces one at a 37.7 ms frame, and the A/B
> is in that ADR. The lesson is worth more than the number: **a rule that refuses is only evidence
> that it works if you can also make it accept.**

### The two photographs

A zoom step, photographed 300 ms after the key — while the real frame is still being built — and
again after it landed.

- **Mid-flight**: the drawing is *already at the new magnification*, moved and scaled. Against the
  photograph taken immediately before the key press it differs by 10.7% RMSE — the window answered
  the input — and against the real frame that replaced it by **1.27%**, which is the blur and the
  detail the old raster never had. Magnified, the difference is visible as softness in the thin
  rules and the banding: the approximation is the real picture, resampled.
- **After**: the same placement, crisp, `presented` in the trace, and the reprojection gone.

## What a revealed edge shows, and why

A zoom out or a scroll moves pixels off the region the old raster covered, and **there is nothing
true to put there**. The reprojection draws only where it has pixels; what shows through elsewhere
is the presenter's own medium — the background every frame already clears to, which in this build
is white.

The alternatives were considered and rejected, and the reason is the same one in both directions.
*Repeating or mirroring the edge pixels* would invent page content. *Painting the region in a
distinct "no information" tone* would put a colour no clause states on the screen at every zoom
out, which reads as a rendering defect rather than as a warning, and would be a louder claim than
the thing it warns about. Drawing nothing asserts nothing that the frame's own background does not
already assert, and it resolves within one frame.

**One consequence stated plainly**: because the medium is white here, a revealed strip is not
visually distinct from blank page for the ~600 ms it lasts. That is the cost of the choice and it
is written down rather than left to be discovered.

## What this costs, beyond the milliseconds

- **One extra scene rebuild per reprojection.** The one-image frame replaces the retained scene, so
  the real frame that follows rebuilds from nothing — which it was going to do anyway, because its
  transform changed. What is genuinely new is one extra tick of `ResourceCaches`' frame clock and
  one extra eviction pass, and the pass does nothing at all unless the device is over half its
  resource budget. On the witness the whole of it is legible in one column: the real frames
  following a reprojection report `settle` of **0.6 to 1.2 ms**, which is the one-image scene's
  transients going back plus that pass.
- **The captured raster crosses the bus twice** — back on the readback, out again as an image
  upload of one window (3.2 MB at 800×1000). It is released the frame after, by
  `drop_unreachable`'s existing rule.
- **Nothing on any judged path changes**, which is the point of rule 2 and is what the gates say.

## What this does not do

- **The processor's window (`--cpu`) is not covered.** There is no retained encode to replay there,
  so its pixels would have to come from rasterising the page again — the cost this exists to hide.
  The attempt is made once, refused, and never repeated. `doc/todo/37` is amended down to that one
  item rather than deleted, because a software surface *does* hold the raster it last presented and
  a reprojection there is a resample rather than a render; it is a smaller piece of work than this
  one and it is written down as such.
- **A page turn is never reprojected.** Nothing about the outgoing page's pixels says anything true
  about the incoming one, at any placement, so `plan` refuses on page identity — the pinned `Arc`,
  for ADR 0351's ABA reason.
- **§12.4.4's transitions are left alone** and clear the record: a transition frame is already a
  picture of two pages moving, and no transform of one is any view of either.
- **It is not progressive rendering.** `doc/todo/16`'s road C is a different feature with a
  different argument, and it draws only marks the file states.

## Clauses

**None.** This is presentation and not a reading: nothing reprojected is a *rendering* of the page,
so §10.7.4's scan-conversion rule does not reach it — it governs how a shape is converted to
pixels, and no shape is converted here. The conformance ledger is unmoved for the same reason, and
that is a statement about this change rather than about the clause.

## What did not move

`fmt`, `clippy --workspace --all-targets`, the workspace test run, the doctests, the conformance
checker, the corpus gate, the oracle, both text gates, dates, XMP, JPEG 2000 and both of
`render-quorra`'s coverage lanes were run to prove that nothing on a judged path changed — which is
rule 2's own gate, since an instrument that started photographing an approximation would show up
there and nowhere else. The session's own file carries the figures.

**One of them was checked by A/B rather than by assertion.** The magnified quorra lane's triple did
not match the previous session's record, so the change was taken off with `git apply -R`, the lane
run again on the bare tree, and the same **937 agree, 10 differ, 4 refused, 23 not comparable**
came back with the same four refusals character for character. The difference is between trees and
not this round's; and the habit is worth keeping, because "the number moved" and "the number is
different from the one written down somewhere" are not the same statement.
