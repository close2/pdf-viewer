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

**One thing is still to be established on a real display**, and it is deliberately not guessed at
here: rule 4 is now the binding constraint, its number is `SHARE` times what a reprojection costs
*on that machine*, and this harness can only measure llvmpipe's — where the readback is expensive
enough to refuse everything after the first. The trace prints the number on the first reprojection
of every run, so **the next report from the owner answers it**.

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
