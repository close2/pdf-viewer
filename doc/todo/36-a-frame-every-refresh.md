# A frame every refresh — 60 Hz as the floor, 120 Hz as the target

Status: **built for the window with a graphics device** (ADR 0383; the trigger and the Wayland
refresh rate corrected by ADR 0384), which is every run without
`--cpu`. The presenter is a clock: the period is the surface's own refresh rate where it states
one and the floor of 60 Hz where it does not, every present is spaced by it, a view that keeps
moving is answered every tick by a reprojection composed against the last *rendering*, a late frame
re-bases, and a still window presents nothing and spends no processor time. The trace's summary
carries the two claims rule 6 asks for: the interval distribution, and what share of the presents
were the page rather than a picture of it moved.
Priority: 36 — the first item in this tree whose acceptance was a *rate*, and it stays open for
the one thing a clock cannot do.
Witness: `tmp/Entwurf.pdf` — **not in the repository**, so no test may name that path. The A/B that
proves the trigger is on `doc/PDF20_AN001-BPC.pdf`, which is.
Instrument: the window's `--trace` frame lines and its summary, which now report a cadence.
Clauses: none — presentation. §10.7.4 does not reach it: nothing reprojected is a rendering.
Code: `crates/viewer-ui/src/bin/pdf-viewer/{cadence,stale,surface,window}.rs`

## The owner's *miss* is now what triggers a reprojection, and it was not (ADR 0384)

The sentence at the top of this file — *"we should still try to render a correct image every frame,
but if we miss, we should interpolate"* — is now the code's own trigger: a **miss** is a frame that
does not land inside `Cadence::period`, which is the surface's own refresh where the surface states
one. ADR 0378's rule 5 had it as `SHARE` × a *measured* reprojection cost instead, and the owner
ran the result on their own graphics device and reported that reprojection did not appear to work.
It did not: the bar was 510 ms until a reprojection had been measured, only a reprojection above
the bar could measure one, and their frames were 80 to 438 ms. **A self-calibrating threshold whose
own gate blocked its only sample.** ADR 0384 has the trace and the A/B.

**And rule 4 was the same defect one layer down, found by the owner running the fix.** With rule 5
re-grounded their second trace reached 7 reprojections of 24 presents — but six view changes still
showed nothing, silently, because rule 4's bar was a *tenth* of the frame and a tenth of a real
device's frame is less than what a readback costs on it. Reprojections of 6 to 16 ms were refused
against frames of 58 to 156. `SHARE` is gone: standing in must **buy at least one refresh**
(`reprojection + period ≤ frame`), which is the smallest difference the display can show. There is
now no number in this design that the project chose rather than measured.

**120 Hz is reached, and this file can finally say so.** The owner's second trace opens
`120.0 Hz — no output claims this window yet, so the slowest display attached states it` and closes
`120.0 Hz, stated by the surface`. Item 3 below was the thing standing in the way and it was a
question asked one moment too early.

## Two small things this round found and did not take

- **The trace does not say how many encode threads quorra got.** `render_quorra::options()` reads
  `available_parallelism` at construction and nothing prints it, so "the encode is parallel here"
  is an inference from the shape of the number rather than a reading of it. ADR 0384 §7 makes the
  inference and says why it holds; a `--trace` line under `Topic::Launch` would make it a fact.
  One line, and it belongs beside `device up in …`.
- **A window dragged to another display keeps the first display's cadence.** `Cadence::ask` stops
  at the first answer from the window's own surface, deliberately (ADR 0384 §5): polling the
  monitor every frame is a per-frame cost for a question that changes when somebody drags a window.
  winit's Wayland backend *receives* `surface_enter` and its handler body is empty
  (`platform_impl/linux/wayland/state.rs:348`), so the honest fix is upstream reporting the change
  rather than this tree polling for it.

## What is left, and it is one thing

**The render runs to completion on the event thread.** `QuorraPresenter::present` blocks, so the
clock decides when a frame may *start* and has no say in how long one lasts: a correct frame of the
witness costs 55–73 ms on this software adapter, which is four refreshes at 60 Hz and eight at 120,
and nothing can be presented during them. It is visible in ADR 0383's own histogram as the single
79 ms interval among ninety-one of one refresh.

The owner's *"we should still try to render a correct image every frame"* is therefore honoured in
the only sense a synchronous renderer permits — a correct frame is attempted at every tick at which
no reprojection is owed — and **not** in the pipelined sense, where a presenter presents at the
deadline whatever a renderer running beside it has finished. That second sense is what is left.

It is a **larger** piece of work than this one was, and the obstacle is named rather than guessed:
`quorra_gpu::Device::render` takes `&mut self` and owns the caches and the surface, so one device
cannot serve a render thread and a present thread at once. Whoever takes it owes the argument for
what crosses the thread boundary before any code — a second device, a channel of finished frames,
or an ask to quorra — and `doc/todo/16`'s road C is the neighbouring item, not the same one.

### The design round settled it, and the answer is an ask (ADR 0386)

*Session 551, a design round: it built nothing. This section is its own and amends nothing above it.*

**The argument owed by the paragraph above is written**, and the three candidates it named are
priced against the owner's own traces rather than weighed. ADR 0386 has the whole of it;
`doc/QUORRA_NONBLOCKING_RENDER.md` is the ask it ends in. What binds here:

- **Per rate, and the asymmetry nobody had stated in one place.** 60 Hz needs a non-blocking render
  and **nothing else about cost**: on the owner's machine all seven of their reprojections fit
  16.667 ms, the worst at 16.2. 120 Hz needs the non-blocking render **and**
  `doc/QUORRA_FEEDBACK.md` §28.6's no-readback path, because six of those seven miss 8.333 ms and
  the readback is the whole of the difference.
- **A third requirement, which this file did not have.** A reprojection needs a *base*, and the
  owner's run printed the refusal twice against its seven reprojections — the atlas repacked and the
  retained encode died with it (ADR 0384 §6). Some view changes can show nothing today whatever the
  presenter does. A page rendered into a texture the host owns is unaffected by a repack, so the
  same change closes this too.
- **`execute` is 0.15% of a frame** — 6.7 ms of 4454.9 over the owner's whole run. The graphics
  device is idle for essentially all of it, which is what declines "submit, then poll" on the
  measurement rather than on a preference.
- **A second device is priced and declined**: quorra has no constructor that adopts a `wgpu::Device`
  and wgpu 30 shares no texture across devices, so the page would cross as **8 192 000 bytes a
  frame** — more than a 120 Hz refresh before anything is drawn.
- **The recommendation is one device on two threads, split at the surface**, and it costs
  `viewer-core` nothing: `NeedsRender`/`RenderReady` already carry an opaque handle "a caller may
  move to a thread of its own", so no `Command`, `Event` or `Query` moves and `interpret` stays what
  it was.
- **What is available without any ask, and was not taken**: deferring the real frame until the view
  comes to rest would give 60 Hz through a gesture today. It stops trying to render a correct image
  every frame, which is the owner's own sentence, so it is theirs to choose. ADR 0386 §3.3.
- **And the ceiling, so the ask is not oversold**: during the 4.366 s the owner's view was moving,
  120 Hz is 524 refreshes and **15 could carry a rendering — 2.9%**. The reprojection is the floor
  and it is most of what would be seen. Every round that makes a frame cheaper moves that number and
  none of them changes the answer to the owner's question.

## What was settled, so that nobody settles it twice

The four questions this file used to carry as unsettled, with where the answer lives:

1. **Where the pixels come from at 8.3 ms** — from a readback taken **once per real frame** rather
   than once per reprojection, and resampled under a recomposed transform after that. The first
   reprojection of a base costs 39.6 ms and the nine after it 3.4–5.9 ms, off one capture and no
   upload. ADR 0383's table. It needed nothing from quorra, and ADR 0382 §6's "the escape hatch is
   complete" is **corrected** in the same ADR: a `Target::Texture` can be rendered into and sampled,
   but presenting it needs the surface, which quorra owns and whose format a host cannot learn
   because `Device` returns no `wgpu::Adapter`. `doc/QUORRA_FEEDBACK.md` §28.6's ask stands and is
   sharper for it.
2. **What "every frame" means when nothing changed** — nothing. The clock is armed by an obligation
   and by nothing else, and a window nobody is touching sits in `ControlFlow::Wait`: measured at
   **no present and no measurable processor time over twenty seconds**.
3. **Where the cadence comes from** — `winit`'s `MonitorHandle::refresh_rate_millihertz`, and it is
   asked **more than once**, which is ADR 0384's correction. Reading it in `resumed` is right on
   X11 and answers `None` on every Wayland session for ever: `Window::current_monitor` on that
   backend is the first output in the surface's own `wl_surface::enter` list, and a Wayland surface
   enters no output until it has been drawn to. So the floor stood in, and **120 Hz was out of
   reach in principle on the platform the owner runs.** The cadence now re-asks after each present
   until the window's own output answers, with `available_monitors`' *slowest* standing in
   meanwhile, and the trace names which of the three said it.
4. **What the gates see** — nothing. Everything this round wrote is in a binary; `doc/todo/37`
   rule 2 is untouched and its test still walks every `.rs` outside `viewer-ui/src/bin`.

**And one the harness could not settle**: `Xvfb` states no refresh rate (`xrandr` reports `0.00`
and `--newmode` does not take), so **120 Hz cannot be observed on this machine at all**. What ADR
0383 establishes instead is the two things that decide it — the presenter sustains 8.3 ms intervals
when the frames fit, and a replay (1.3–1.5 ms) and a composed reprojection (3.4–5.9 ms) both fit
while a correct frame of this page does not. A run on the owner's own display is what would close
it.

## Why the owner's framing is right, and worth keeping

*"We should still try to render a correct image every frame"* — the reprojection is a floor under
the experience, never a substitute for the frame being fast. Everything that makes a frame cheaper
(ADR 0377's threads, ADR 0374's raster cache, ADR 0351's retained frame) reduces how often this
item is visible, and it has moved a long way already: ADR 0378 measured this witness's zoom step at
492–1036 ms and this round measured it at 55–73. The standing intent from
[`37`](37-a-frame-that-says-it-is-stale.md) holds — **the better the renderer gets, the less this
should ever be seen** — and it is now legible as a number, because the summary says what share of
the presents were correct.
