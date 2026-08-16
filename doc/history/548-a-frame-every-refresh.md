# 548 — A frame every refresh: the presenter becomes a clock

2026-08-16. `doc/todo/36`, the project owner's own item. ADR 0383.

## What was built

`crates/viewer-ui/src/bin/pdf-viewer/cadence.rs` — new — and changes to `stale.rs`, `surface.rs`,
`window.rs`, `timing.rs`, `app.rs` and the binary's crate root. **No library changed**, which is
`doc/todo/37` rule 2 and is why every judged gate below is unmoved by construction rather than by
luck.

- **The presenter is a clock.** The period is the surface's own refresh rate (`winit`'s
  `MonitorHandle::refresh_rate_millihertz`, read in `resumed`) and `doc/todo/36`'s 60 Hz floor
  where the platform states none. `redraw_requested` defers a frame that arrives before the
  surface has refreshed; `about_to_wait` waits until the tick when a frame is owed and waits for
  an *event* when none is. A window that has been still is due at once, so input latency is
  unchanged.
- **Compose, do not chain.** `Settled` gained a `base` — the last **real** frame's pixels, read
  back once — and `plan` composes `settled⁻¹ ∘ asked` against that rendering whatever is on the
  screen. The capture is asked for only while the window is showing a rendering, and the pixels
  leave the module only through `Stale::reproject`, so no caller can resample anything else.
- **A late frame re-bases**, because the base lives inside the `Settled` a landing frame replaces.
- **Two more call sites moved onto the clock**: the frame replacing a reprojection (`MustFollow`
  now discharges to the cadence, not to `request_redraw`), and §12.4.4's transition, which was
  `ControlFlow::Poll` — a core at 100% for its length — and is now one frame per refresh.
- **The trace's summary reports a rate**, which is new for this project: the cadence asked for and
  where it came from, the interval distribution (median, p90, p99, max, and how many landed on the
  next refresh), and how many presents were the page rather than a picture of it moved.

Seven tests added — four in `cadence`, three in `stale` (composition against the base, the base
captured once per real frame, a late frame re-basing).

## What was measured

`Xvfb` at 900×1100, llvmpipe, release binary of this tree, `tmp/Entwurf.pdf`, driven by `xdotool`.
The surface states no refresh rate (`xrandr` reports `0.00`), so every run asks for the 60 Hz floor.

- **200 `+` keys at 8 ms** — input at 120 Hz: **97 presents, 87 a rendering and 10 a reprojection
  (89.7% correct)**; intervals median 16.7, p90 16.9, **94 of 96 on the next refresh**. The driven
  stretch off the trace's own clock: 89 of 95 intervals in [16, 18) ms, one at 73.
- **Ten reprojections off one base**: the first 45.1 ms (readback 31.2, one upload), the nine after
  it **3.7–5.3 ms with no readback and no upload**. A factor of 11.6.
- **Idle**: twenty seconds with no input — no present, and no measurable processor time (0 ticks).
- **The A/B that names the clock**: the same script with the redraw gate off gave a median interval
  of 8.3 ms — the window ran at the *input's* rate — and the same input with the gate gives 16.7.
- **`--cpu`** drives correctly through the same gate: 15 presents, all correct.

ADR 0383 has the tables, the argument, and what a clock cannot do on a thread the render blocks.

## What was corrected

**ADR 0382 §6's "the escape hatch is complete and needs nothing from upstream" is wrong in one
half.** A host can have quorra render into a `Target::Texture` it owns and sample it; it cannot
*present* one, because quorra owns the surface, keeps it private, and returns no `wgpu::Adapter` —
and both ways of learning a supported surface format take one. `doc/QUORRA_FEEDBACK.md` §28.6's ask
stands and is sharper for it. This round needed neither, because a readback amortised over one real
frame is cheaper than a new present path.

## Gates

Every one run, all green. `fmt` clean; `clippy --workspace --all-targets` silent of lints;
**2032 tests run, 2032 passed, 15 skipped** (7 new); doctests pass; sandbox binaries built; corpus
gate passes; oracle passes (2 tests); both text gates pass (4 tests, 98.26% of matched words in
bounds); conformance passes (5 tests, 137 under nextest); `render-quorra`'s default lane passes and
its **gpu lane at scale 4** passes over 974 pages. Binaries installed under `target/`.

One gate caught a real mistake in this round's own prose: `every_citation_names_a_clause_that_exists`
rejected a `§` used to name a section of ADR 0383 inside `cadence.rs`, which is exactly the confusion
it exists to prevent.
