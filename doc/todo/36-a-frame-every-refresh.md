# A frame every refresh — 60 Hz as the floor, 120 Hz as the target

Status: **built for the window with a graphics device** (ADR 0383), which is every run without
`--cpu`. The presenter is a clock: the period is the surface's own refresh rate where it states
one and the floor of 60 Hz where it does not, every present is spaced by it, a view that keeps
moving is answered every tick by a reprojection composed against the last *rendering*, a late frame
re-bases, and a still window presents nothing and spends no processor time. The trace's summary
carries the two claims rule 6 asks for: the interval distribution, and what share of the presents
were the page rather than a picture of it moved.
Priority: 36 — the first item in this tree whose acceptance was a *rate*, and it stays open for
the one thing a clock cannot do.
Witness: `tmp/Entwurf.pdf` — **not in the repository**, so no test may name that path.
Instrument: the window's `--trace` frame lines and its summary, which now report a cadence.
Clauses: none — presentation. §10.7.4 does not reach it: nothing reprojected is a rendering.
Code: `crates/viewer-ui/src/bin/pdf-viewer/{cadence,stale,surface,window}.rs`

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
3. **Where the cadence comes from** — `winit`'s `MonitorHandle::refresh_rate_millihertz`, read once
   when the window exists. The floor stands in where the platform states none, and the trace says
   which.
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
