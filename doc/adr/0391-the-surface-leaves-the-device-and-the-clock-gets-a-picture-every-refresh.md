# 0391 — The surface leaves the device, and the clock finally gets a picture

**Status.** Accepted. Session 556. Builds `doc/todo/36`'s remaining item and closes the ask ADR
0386 ended in. Rests on ADR 0383 (the presenter as a clock, and the one base it composes against),
ADR 0384 (the two thresholds re-grounded on the display's own unit), ADR 0385 (the base that
outlives its frame), ADR 0386 (the design round that priced the options and recommended this one)
and quorra's **ADR 0056**, which is the change that made it possible.

`doc/QUORRA_NONBLOCKING_RENDER.md` is the ask; `doc/QUORRA_NONBLOCKING_RENDER_ANSWER.md` is
quorra's reply, carried across verbatim and not edited by this tree.

## The answer was yes

Four sessions asked one question — *"will we be able to achieve either correct or reprojected
frames for every frame?"* — and got the same answer four times: **not while there is one
`&mut Device` between the clock and the screen.** `quorra_gpu::Device::render` ran to completion
on the thread that owned the event loop, and it owned the surface, so for the 57.7 to 913.1 ms a
frame of the owner's document costs, nothing could be presented, no input could be read, and the
reprojection that exists precisely for those milliseconds could not be issued — it needed the same
borrow.

quorra took the ask and built it. `Device::detach_presenter() -> Option<Presenter>`, a `Send`
`Presenter` holding the surface, its swapchain and one pipeline, and
`Presenter::present(&[Layer])` where a `Layer`'s `placement` maps the layer's own texel space to
the surface's pixels. Their §4 answered our §4 almost unchanged, their §7(b) settled the soundness
question we said we could not settle from outside, and their §5 accepted all four of our reasons
for asking them rather than doing it here.

**So this round is the caller. Everything below is what it took to be one.**

## 1. The arrangement

| | the event thread | the render thread |
|---|---|---|
| holds | `quorra_gpu::Presenter` | `render_quorra::QuorraWindowRenderer` — the device, the caches, the retained scenes |
| does | acquires the swapchain, draws three textured quads, presents | walks display lists, encodes, draws into two textures |
| never does | encodes, uploads, reads back | touches a window, a `Viewer` or an `App` |

What crosses is a job one way and a finished frame the other, plus **the texture pair travelling
back and forth**: the event thread returns whichever pair it has stopped showing with the next job,
so a window's 8 192 000 bytes are allocated at bring-up and at a resize and never per frame.

**One job in flight at a time**, deliberately. A queue would fill at the tick rate while a frame
of this page takes thirteen ticks, and every job in it would be answering a view the person had
already left. When a frame lands and the view has moved, the next job is asked for *then*, with
the view as it is by then.

### Three layers, in this order

1. **the medium** — one opaque texel scaled over the window. A page moved under a new view reveals
   what it does not cover, and ADR 0378 decided years of sessions ago what belongs there: the
   window's background, never page white, because white would assert that the page is blank. A
   window-sized medium texture would spend a window's bytes to hold one colour.
2. **the page**, under the placement `crate::stale` computes — the identity where the frame on hand
   is of the view being asked for, `settled⁻¹ ∘ asked` where it is not. Sampled linearly: at the
   identity the sampler lands on texel centres and the two filters agree exactly, so a rendering
   pays nothing for it, and a stand-in is a blur rather than a grid of squares (ADR 0378's choice,
   unchanged).
3. **the chrome**, at the identity, on transparency.

**The chrome is a second texture and that is the design rather than an implementation detail.** A
host that put the page and the chrome in one raster could only ever move both, and moving the page
while keeping the sidebar still is the whole difference between a viewer that feels responsive and
one that wobbles.

## 2. What ADRs 0383 to 0385 established, and what became of each

| | before | now |
|---|---|---|
| the clock is the surface's own refresh (0383) | unchanged | unchanged |
| compose, never chain (0383) | `Stale::composed` reads the base's placement | `Stale::reproject` hands back a **placement and no pixels at all**, so a caller cannot supply either half |
| a late frame re-bases (0383) | `Stale::settled` replaces the whole record | unchanged, and now the pixels are replaced by the same event |
| every refusal says *impossible* or *unwise* (0385) | seven kinds | six: one variant became unreachable and was deleted rather than kept |
| the base outlives its frame (0385) | an `Arc<[u8]>` read back off the window | **the texture the device rendered into**, which cannot be lost |
| rule 5 is a miss against the period (0384) | predicted from the last built frame | predicted **and observed** |
| rule 4 is `reprojection + period ≤ frame` (0384) | a bound on what standing in costs the real frame | **deleted**, §4 |

### The base cannot be lost, and two of the owner's refusals go with the route

ADR 0385 fought a defect that the owner reported twice: a real frame that repacked the glyph atlas
destroyed the retained encode, so the readback a reprojection needed had nothing to replay and the
window did not move. That round made the base outlive its frame, which was the right fix for the
arrangement it was in.

**There is no readback now.** The base is the texture the device drew the page into, and an atlas
repack throws tile placements away without touching a pixel already drawn. So the refusal is gone
with the route that produced it rather than handled — which is the stronger of the two outcomes,
and `a_repacked_atlas_can_no_longer_refuse_a_view_change` is the property stated as a test.

It also takes two costs out of every stand-in that ADR 0386 §1 measured on the owner's machine: a
readback of 2.7 to 6.6 ms against a 8.333 ms refresh, and an 8 192 000-byte re-upload behind it.

### Rule 5 gains a second way of knowing, and it is the one that cannot be wrong

The owner's word is *miss*, and until this round a miss could only be **predicted**, from what the
last frame that built a picture cost. A prediction is what answers the very first tick of a view
change — nothing has been asked for yet, so there is nothing to observe.

With the render on a thread of its own there is a second instrument: **a render still being drawn
when the next tick comes round has missed that refresh**, whatever anybody predicted. It needs no
calibration, no first sample and no constant. Both are asked; either standing in is enough.

The case that needs it is a page whose last frame was quick — so the prediction says the next will
land in time — and then it does not. Before the split that state could not exist, because a frame
ran to completion inside the tick that started it.

## 3. What "waiting" means now, and why rule 5 still refuses

A refusal used to mean *draw the real frame instead*, which the event thread then did. It now
means **present nothing this tick and keep the clock armed**, so the tick that follows carries the
rendering this one waited for.

That is the owner's sentence honoured rather than weakened: *"we should still try to render a
correct image every frame, but if we miss, we should interpolate."* A frame expected inside one
refresh is *tried for*, and the cost of trying is at most one refresh of latency — against a
resampling of a frame that was about to arrive anyway.

## 4. Rule 4 is deleted, and this is the argument rather than the attrition

ADR 0384 re-grounded it: standing in had to **buy** a whole refresh, `reprojection + period ≤
frame`, because a reprojection ran on the event thread and pushed the real frame back by exactly
what it cost. The rule was right and its premise was true.

**The premise is now false.** A reprojection is three textured quads issued by the thread holding
the surface, while the render runs on another; what it costs the frame it stands in for is nothing
at all rather than a small fraction. A bound on a cost that is structurally zero is not a bound.

So it is removed, and with it `Stale::measured`, `Stale::affordable` and `Refusal::TooDear`. The
discipline is the one that put the rule there in the first place: **a number nobody can name a
purpose for is a number to remove.** What a stand-in costs is still *reported*, on the frame line,
where a person reads it — what is gone is a gate reading it.

`a_frame_that_misses_by_a_hair_is_still_stood_in_for` is what that leaves as a property: at every
cost the owner's own traces recorded, including a frame that overruns the refresh by a fraction of
a millisecond, a view change is stood in for. Under rule 4 that last one was refused and the window
did not move.

## 5. What `doc/todo/37` rule 2 required, and how it is still structural

Rule 2 is that nothing which judges a picture can ever see an approximate one, and it is
structural rather than careful: everything that makes one is in a module of `pdf-viewer`'s binary,
which is a dependency of nothing.

**This round strengthened it rather than spending it.**

- **The offscreen rasteriser did not move.** `QuorraRasterizer::rasterize_frame` renders through
  `Target::Readback` exactly as it did; the corpus gate, both oracle lanes and every diagnostic
  artefact are untouched. The one shared function, `render_quorra::present::build`, gained an
  `Option<Color>` medium and every existing caller passes `Some(background)` — so no golden moved,
  which the four corpus lanes then confirmed rather than assumed.
- **Nothing outside `viewer-ui/src/bin` learns what a reprojection is.** `render-quorra` draws a
  window's frame into two textures and hands them back; `crate::renderer` puts them on a window
  under placements `crate::stale` computes. Neither library knows why a placement is not the
  identity, and `no_library_in_this_tree_knows_what_a_reprojection_is` still walks every `.rs`
  outside that directory and still passes.
- **`Stale` hands out a `Transform` and no pixels.** It used to hand out a one-image display list.
  A caller can now neither supply the pixels to resample nor the placement to compose against,
  which is one fewer way for "compose, do not chain" to be got wrong.

## 6. The startup rules, item by item

`CLAUDE.md` binds three things here and forbids none of the arrangement.

- **`detach_presenter` is on the launch path, and that is checked rather than assumed.** quorra's
  own documentation says it clones four handles and moves the surface state, asking the pipeline
  store nothing; their §6 says it cannot compile, cannot wait for warmth and cannot block, with a
  proof under Xvfb asserting `compiled == None` on a device that was waited warm. The presenting
  pass is in the warm set of every device built for a surface, so this tree's launch path pays no
  shader for it — and where the warm-up thread has not reached it, the **first present** compiles
  it inline and says so in `PresentCost::compiled`, which is ADR 0043's rule unchanged.
- **The render thread is spawned by the first job, not in `resumed`.** A spawn in `resumed` would
  put a scheduler decision in front of `graphics device`, which is a launch milestone.
- **Nothing joins it, waits for it or blocks on a first frame.** The first present is the first
  render's, arriving when it arrives; the launch timeline's *content* is unchanged, and
  `first scene built` is marked when the frame lands rather than when it was asked for, because
  that is now the moment it is known.

## 7. What this cost

- **Two textures instead of none**, 8 192 000 bytes each at the owner's window size, plus the pair
  in flight — so up to four windows' worth of pixels resident where there were none. That is the
  price of a page that can be moved without being redrawn, and it replaces a readback buffer and
  an upload of the same size per stand-in.
- **The chrome is exactly as stale as the page during a stand-in**, and it was not before: a
  reprojection used to redraw the chrome as geometry over the resampled page. quorra's `Layer`
  documentation anticipates this — "the chrome stays exactly as stale as the page and no worse" —
  and the alternative is a chrome render on the presenting thread, which is the encode this whole
  arrangement exists to get off it. Written down as a cost rather than discovered later.
- **A page turn now costs a render before anything is shown of the new page**, where before the
  event thread drew it and presented it in one call. It is the same wait, moved; what changed is
  that input is answered during it.
- **One thread**, spawned once, holding the device for the life of the window.

## 8. The measurement, on the owner's own display

`tmp/Entwurf.pdf` — 58 009 display commands, not in the repository and named in no test — in a
1275×1594 window on the project owner's AMD Radeon 890M under RADV, on a display that **states
120 Hz itself**: `presenting on a cadence of 120.0 Hz (8.336 ms), stated by the surface`. XWayland,
because `xdotool` cannot reach a Wayland client. Two gestures, sixteen zoom steps each.

### The rate, which is the question

| | session 549, before | **paced**, a step every 1.5 s | **held**, a step every 0.12 s |
|---|---:|---:|---:|
| presents | 24 | **533** | **309** |
| median interval | **167.4 ms** | **8.4 ms** | **8.3 ms** |
| p90 | 735.9 | 13.7 | 11.1 |
| on the next refresh | **1 of 23 — 4.3 %** | **468 of 532 — 88.0 %** | **290 of 308 — 94.2 %** |
| a rendering of the page | 17 (70.8 %) | 17 (3.2 %) | 3 (1.0 %) |

**The median interval is 8.4 ms against a stated refresh of 8.336.** Over the stretches where the
view was actually moving — every interval under a second, so the idle gaps between gestures are
excluded — the paced run is 516 intervals with a median of 8 ms and 90.7 % inside a refresh and a
half; the held run is 307 with a median of 8 and 94.1 %.

The tail is honest and worth naming: 14 intervals of 25–50 ms in the paced run and 4 over 50 in the
held one. They are ticks the event thread spent behind an acquire, and they are the whole of the
distance from 94 % to 100 %.

### What a picture costs on the presenting thread

The frame line's new `present` column — quorra's `PresentCost`, its three wall clocks summed:

| | median | p90 | max |
|---|---:|---:|---:|
| a reprojection, paced (n = 516) | **0.51 ms** | 6.10 | 44.68 |
| a reprojection, held (n = 306) | **0.37 ms** | 4.33 | 78.55 |
| a rendering put up (n = 17) | **0.23 ms** | 0.40 | 1.69 |

**Half a millisecond of a 8.336 ms refresh**, where session 549's reprojection on the same machine
was 6.2 to 16.2 ms with a readback in it and an 8 192 000-byte upload behind it. The p90 and the max
are an `acquire_wall` waiting for the presentation engine rather than work this program does — which
is `PresentCost::reconfigured`'s territory and a thread to pull if the tail ever matters.

### The correct-frame share, and it is the number the report must not blur

**3.2 % paced, 1.0 % held.** That is *lower* than session 549's 70.8 %, and it is not a regression —
it is the same denominator finally being counted. Before, a present happened only when a frame
finished, so almost every present was a rendering and there were twenty-four of them in a minute.
Now there are 533, of which 17 are renderings, because **the other 516 refreshes got a picture where
they used to get nothing at all.**

quorra's reply §9 predicted this to within a fraction: during the owner's earlier 4.366 s of
movement, 120 Hz is 524 refreshes and 15 of them could carry a rendering — **2.9 %**. This round
measured **3.2 %**. The ceiling was right, and it is the frame's rather than the presenter's.

### Two refusals that did not happen

`0 view change(s) showed the real frame instead: 0 had nothing true to move and 0 were a judgement
between two measurements`, over sixteen view changes and 632 ticks — **and the atlas was repacked
after 4 of those frames.** Under the readback route each of those four would have refused a view
change for want of a capture, which is the defect ADR 0385 was written about and the owner reported
twice. It cannot happen now, and this run is where that stops being an argument.

### The launch path did not move

`graphics device 32.3 ms` (session 549's trace: 32.7), `first present 1683 ms` of which 778 is
interpreting 58 009 commands and 328 the first scene walk. Detaching the presenter is on that path
and cost nothing measurable, which is what quorra's four handle clones predict.

## 9. What this does *not* buy, said plainly

**A rendering every refresh, and it never could.** quorra measured the floor from their side and
the answer is in their §9: with the whole of `encode` at zero this page is still **107.0 ms a
frame**, and with everything quorra does costing nothing our own scene walk alone is **24.4 ms —
2.9 refreshes**. This page cannot be drawn inside one refresh by any arrangement of any code either
project could write.

So the target this round is measured against is **a picture every refresh**, not a rendering every
refresh, and the report must not blur the two. What the split changes is that the refresh gets a
picture *while* a frame is still running — which is the only thing that was missing, and the only
thing that could have been.

## Clauses

**None**, for ADR 0378's, 0383's, 0384's, 0385's and 0386's reason unchanged: this is presentation
and not a reading. Nothing reprojected is a *rendering* of the page, so §10.7.4's scan-conversion
rule does not reach it, and the conformance ledger is unmoved. What a page looks like is decided by
`render-quorra`'s translation and quorra's encode, and neither moved.
