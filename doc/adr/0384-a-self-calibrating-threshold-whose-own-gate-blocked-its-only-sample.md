# 0384 — A self-calibrating threshold whose own gate blocked its only sample

**Status.** Accepted. Session 549. A defect round on the project owner's own report. Corrects
ADR 0378's rule 5 in place and amends ADR 0383 §4, which had already named the shape of the
problem and declined to take it. Rests on ADR 0383 (the cadence), ADR 0351 (the retained frame)
and ADR 0350's `Counters` (the atlas instrument, on real hardware for the first time here).

## Context

The project owner ran the feature ADR 0383 built on their own machine — AMD Radeon 890M under
RADV, Wayland — and reported one sentence:

> I don't have the impression that reprojection works.

Their trace ends:

```text
15 present(s): 15 a rendering of the page and 0 a reprojection of one (100.0% correct)
intervals between presents, ms: median 362.0  p90 707.5  p99 2187.7  max 2187.7
                               — 0 of 14 on the next refresh (0.0%)
```

Thirteen zoom steps, frames of 80.8 to 437.9 ms, and **not one reprojection**. The feature was
built, tested, measured over two sessions, and did nothing at all on the machine it was for.

## The diagnosis, verified rather than repeated

`Stale::threshold()` was `measured.unwrap_or(ASSUMED) * SHARE`. `measured` is set in exactly one
place — `Stale::drawn`, which records a reprojection that has just been drawn — and a reprojection
was drawn only where `settled.cost >= threshold()`. So until one had been drawn the bar was
`ASSUMED × SHARE` = 51 ms × 10 = **510 ms**, and the only thing that could lower it was the thing
it gated.

**A self-calibrating threshold whose own gate blocks its only sample.** On any machine quicker
than the llvmpipe adapter `ASSUMED` was measured on, the bar can never come down. It is not a
tuning error: no value of `ASSUMED` fixes it, because the failure is in the direction of the
dependency.

Read off the owner's trace, frame by frame, it is exact:

| at | what happened | `settled.cost` when the next view change asked | bar | outcome |
|---|---|---:|---:|---|
| 1.611 | launch frame, `encoded` | — | 510 | no base yet |
| 1.799 | `Focused(true)` redraws the same view, `replayed` | 778.6 | 510 | refused: the view did not move |
| 3.586 | first zoom | **2.1** | 510 | refused |
| 3.986 … 8.387 | twelve more zooms | 80.8 – 437.9 | 510 | refused, every one |

Two things had to be true for it to produce nothing, and both were:

1. **The bar was 510 ms and the frames were 80 to 438.** Never met.
2. **The one frame that would have met it never got the chance.** The launch frame cost 778.6 ms
   — over the bar — but a window-focus event redrew the *same view* before the first zoom, quorra
   replayed the retained encode in 2.1 ms, and `Stale::settled` overwrote the cost with that. The
   bootstrap sample was destroyed by a frame that drew nothing new.

The second is a defect in its own right and is fixed separately below, because it survives the
first fix: a *replay* measures the replay.

## Decision

### 1. Rule 5 is the cadence's own period, which is what the owner asked for

Their words, in `doc/todo/36`:

> We should still try to render a correct image every frame, but if we miss, we should interpolate
> from the last frame (even if the last frame was already incorrect).

A **miss** is a frame that does not land inside the cadence's own period. That number is the
presenter's already — `Cadence::period`, the surface's own refresh where the surface states one
and `doc/todo/36`'s floor where it does not — and it is a measurement rather than a taste. It
needs no calibration, no bootstrap, and no first sample.

```text
rule 5:  the frame this view is waiting for is expected to cost more than one refresh
```

That is the whole of it, and the property it has that the old bar did not is that **nothing which
gates a reprojection depends on a measurement only a reprojection can produce**. A test states it
directly.

### 2. Rule 4 stays, separately, and unmeasured now *permits*

`doc/todo/37`'s rule 4 — "if the reprojection cannot be produced within a small fraction of the
frame it replaces, it is not produced at all" — is a real requirement and is kept as its own
check, `Stale::affordable`: `SHARE` times what a reprojection has **actually cost on this
machine**, against the frame it stands in for. What changed is the state before there is a
measurement. It used to refuse; it now permits.

That is not a weakening, and the argument is the one the defect makes for itself: rule 5 has
already established that this frame will miss its refresh, the first reprojection is the only way
this machine's number can ever be learned, and it is **one** frame — measured, printed by name in
the trace, and binding on every reprojection after it. A bound that refuses until it has a
measurement it can only obtain by not refusing is not a bound; it is an off switch that looks like
a policy.

**What that exposure costs is one bad trade per run, and it is bounded.** On the harness below the
first reprojection cost 30.4 ms to stand in for a 37.7 ms frame — 81% of it, a poor deal — and
rule 4 then refused every later one on that document. That is the design working: the bad trade is
made once, at a cost of tens of milliseconds, and it buys the number that prevents the rest.

**Rule 4 is a per-frame check and deliberately not a run-level refusal.** The two run-level
refusals that exist (`Stale::refuse` — no device to read back from, and a capture that re-encoded)
are facts about the *machine*. A ratio between one reprojection and one frame is a fact about a
*frame pair*, and letting one marginal pair switch the feature off for a session would be the same
class of defect this ADR repairs, arriving from the other direction.

### 3. A replayed frame does not speak for what a render will cost

`Settled::cost` is gone; `Stale::building` replaces it, and it is updated **only by a frame that
built its picture**. Whether a frame built one is quorra's own observable — `FrameCost::encode_source`
— rather than an inference from a small duration, and the test is against `Replayed` rather than
for `Encoded`, so a frame that never reached the device (the processor's window, the fallback) is
correctly counted as having built one.

The justification is not the owner's trace, though that is where it was found: **a view change can
never replay**, because the placement is part of `render-quorra`'s scene key. So a replay is
evidence about a class of frame that rule 5 is never asking about, and the previous *present* is a
predictor of the next *render* only when nothing harmless was redrawn in between. A window focus, a
caret, a selection — every one of them resets the estimate to a couple of milliseconds under the
old shape.

Moving the cost off `Settled` is also the right place for it on its own terms: a cost belongs to
the machine, the pixels and their placement belong to one frame, and ADR 0383's invariant that the
latter two are replaced together is kept intact by the split rather than in spite of it.

### 4. The refusals say so, which two of them did not

`capture_presented` returns `Ok(None)` where there is no retained encode to replay, and the host
returned `false` in silence. So did a raster in a layout this host cannot read. **A refusal a
person cannot see is indistinguishable from a feature that does not work**, which is precisely the
report this round was opened by, so both now print a line. They are cheap: nothing reaches them
except a view change that rules 4 and 5 have already admitted.

## What the harness can establish, and what it cannot

`Xvfb :NN` at 900×1100, llvmpipe, release binaries of this tree before and after, driven by
`xdotool`: ten `+` and six `-`, spaced 250 ms so each step waits for the frame before it.

**The A/B is on `doc/PDF20_AN001-BPC.pdf`, which is in the repository**, and it is the case this
round was told to construct — a frame that reliably exceeds one refresh while staying far below
510 ms:

| | presents | reprojections | frames, median / p90 / max |
|---|---:|---:|---|
| before | 17 | **0** | 15.0 / 21.3 / 37.6 ms |
| after | 18 | **1** | 7.3 / 30.5 / 37.7 ms |

and the line that says why:

```text
approximated: this view's frame is expected to cost 37.7 ms against a 16.7 ms refresh, so it
misses, and the last rendering's own pixels stand in (read back in 9.4 ms, whole reprojection
30.4 ms); the real frame has been asked for
```

**37.7 ms is fourteen times below the bar that used to be there.** ADR 0378 recorded this same
document producing "six frames and zero reprojections" and read it as rule 5 working; it was rule
5 unable to fire.

Three things this harness **cannot** say, stated plainly because the harness is what hid the
defect in the first place:

- **How often rule 4 then permits one on the owner's machine.** On llvmpipe a reprojection costs
  30 to 37 ms of which the readback is 9 to 24, so `SHARE × measured` lands at 300–372 ms and every
  later frame of every document tried here is refused. On the owner's own witness
  (`tmp/Entwurf.pdf`, not in the repository) both binaries therefore produce one reprojection of
  eighteen presents — the same one, unlocked by the launch frame — and the harness shows no
  difference at all. What decides it there is what a readback costs on a real device against
  frames of 80 to 438 ms, and his trace will now print that number on its first reprojection.
- **Anything about Wayland.** `Xvfb` is X11 and states no refresh rate by either route, so both
  binaries take the floor and the correction in §5 below is exercised by unit tests and by reading
  winit, not by running.
- **Whether 120 Hz is reached.** Unchanged from ADR 0383 and for the same reason.

## Three secondary findings from the same trace

### 5. The surface states no refresh rate on Wayland, and it is not the platform's fault

`Cadence::of` was called once, in `resumed`. On winit 0.30.13's Wayland backend
(`platform_impl/linux/wayland/window/mod.rs:636`) `Window::current_monitor` is the first output in
the surface's own `wl_surface::enter` list — and **a Wayland surface enters no output until it has
been drawn to**, which winit's own platform note says outright (`platform/wayland.rs:3`). `resumed`
is strictly before the first present. So on every Wayland session the answer was `None`, the floor
stood in, and 120 Hz was unreachable in principle.

The rate itself is there and is populated: `MonitorHandle::refresh_rate_millihertz`
(`wayland/output.rs:78`) reads the `wl_output::mode` event's `refresh`, which the compositor sends
for every output at bind time. The question was asked one moment too early.

Two routes, in the order that answers best, and the cadence now asks again after each present
until one of them does:

1. `current_monitor` — the output this window is on. Once it answers, the asking stops.
2. `available_monitors`, which Wayland populates from the start. It cannot say which display this
   window is on, so the **slowest** is taken: `doc/todo/36`'s own argument makes presenting faster
   than the display the failure, so an unknown period is the longest candidate rather than the
   shortest.

The three sources are now three sentences in the trace rather than a two-state flag, because "we
present at 60 Hz", "this window's display refreshes at 60 Hz" and "the slowest display attached
refreshes at 60 Hz" are three different claims.

**What this deliberately does not do is follow a window between displays.** It stops at the first
surface answer. A per-frame monitor query is a cost on every frame for a question that changes when
somebody drags a window, and the honest fix is winit reporting the change — its Wayland backend
receives `surface_enter` and the handler body is empty (`wayland/state.rs:348`). Written down as a
limit, with its witness, rather than papered over.

`primary_monitor` is `None` on Wayland by construction and is no route at all.

### 6. The atlas repacked after 3 of 15 frames, and it is not the explanation for 1 of 15 replays

The two numbers are unrelated and the trace invites conflating them.

**The replay count is right and expected.** Thirteen of the fifteen frames were zoom steps, a
placement is part of `render-quorra`'s scene key, so every one of them *must* re-encode. The single
replay is the `Focused(true)` redraw of an unchanged view. There is nothing wrong here: a run of
zoom steps has nothing to replay, by design.

**The repacks are also ordinary, and the working set proves it.** quorra repacks when the atlas
holds tiles this frame did not use and this frame's set would fit without them. A new magnification
is a new set of glyph rasters, so every zoom step orphans the previous step's tiles; three of
thirteen crossed the condition. The busiest frame's working set is 3 643 223 bytes against
`DEFAULT_ATLAS_BUDGET` of 8 MiB — **the page fits with room to spare**, so this is not the
thrashing pathology `Counters::atlas_repacked` was added to make visible and raising the budget
would not change it. This is that counter's first witness on real hardware and the answer it gives
is "nothing to do", which is worth recording as much as an alarm would be.

**But it costs reprojections, and that is the actionable part.** `capture_presented` returns
`Ok(None)` when the last frame repacked, because the retained encode died with the tile placements.
So three of the owner's thirteen view changes would find no base *even with rule 5 fixed*, and
before this round they would have found it in silence. They now say so (§4). No ask goes to quorra:
the behaviour is correct, the counter already reports it, and `doc/QUORRA_FEEDBACK.md` has nothing
to add.

### 7. Encode threads: the number reaching quorra is the one we think

`render_quorra::options()` sets `encode_threads: available_parallelism()`, and both window
constructors use it — `QuorraPresenter::new` and `with_instance` each spread
`&crate::options()` (`present.rs:561`, `:576`), so the window path is not one that quietly took
`Options::default()`'s 1. quorra clamps the number where the device is built and holds it to
`available_parallelism` itself (`quorra-gpu/src/startup.rs:205`), enters a `std::thread::scope`
inside `Device::render` and leaves nothing behind, so nothing is on the launch path.

The owner's `device` median of 258 ms with `encode` at 128.9 is therefore **not** a single-threaded
encode. It is consistent with ADR 0377's own table, where this drawing's encode falls from 467 ms
on one thread to 221 on eight and 151 on twenty-four: 128.9 ms at 58 009 commands on a machine of
that class is the multi-threaded column, not the serial one. What the trace does not carry is the
number itself, which is the one gap worth naming — `available_parallelism` is read at construction
and never printed, so the claim above is an inference from the shape rather than a reading. A
`--trace` line for it is a one-line item and is recorded in `doc/todo/36` rather than taken here.

## What this costs

- **One unmeasured reprojection per run**, priced above: bounded, reported, and the only way the
  machine's own number can be obtained.
- **One `Arc` bump per present until the surface states its rate** — on Wayland, exactly one
  frame; on a platform that answers at `resumed`, none at all.
- **`Settled` lost a field and `Stale` gained one.** Net state is unchanged and the invariant ADR
  0383 built — pixels and placement replaced together — is untouched.
- **Nothing on any judged path changes**, which is `doc/todo/37` rule 2's own gate. Every gate was
  run; both quorra lanes came back character for character.

## Clauses

**None**, for ADR 0378's and ADR 0383's reason unchanged: this is presentation and not a reading.
Nothing reprojected is a *rendering* of the page, so §10.7.4's scan-conversion rule does not reach
it, and the conformance ledger is unmoved.

## What did not move

`fmt`, `clippy --workspace --all-targets`, the workspace test run, the doctests, the conformance
checker, the corpus gate, the oracle, both text gates, dates, XMP, JPEG 2000 and both of
`render-quorra`'s coverage lanes. The session's own file carries the figures. **No library
changed**: every line of behaviour this round wrote is in
`crates/viewer-ui/src/bin/pdf-viewer/`, a binary, and the test that walks every `.rs` outside it
still passes.
