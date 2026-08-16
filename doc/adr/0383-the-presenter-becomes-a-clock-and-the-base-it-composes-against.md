# 0383 — The presenter becomes a clock, and the one base every reprojection composes against

**Status.** Accepted. Session 548. Builds `doc/todo/36`. Rests on ADR 0378 (the reprojection and
its five rules, all of which still bind), ADR 0351 (the retained frame), ADR 0374 and ADR 0377
(what made the correct frame cheap enough for this to be worth measuring), and ADR 0382 §6, whose
central claim it corrects.

## Context

The project owner's words, quoted in `doc/todo/36`:

> I want it to be able to render every frame (at least 60Hz but 120Hz should be the target). … We
> should still try to render a correct image every frame, but if we miss, we should interpolate
> from the last frame (even if the last frame was already incorrect). If possible and a frame is
> delayed we could use the delayed frame for further interpolated frames.

ADR 0378 answered the *first* frame after an input and nothing after it: one reprojection, then the
window waited for the real frame however long it took. The three things this asks for beyond that
are a **cadence**, a **second reprojection** that does not compound the first, and a **late frame
that re-bases**.

## Decision

### 1. The presenter becomes a clock, and the clock is a rate limit

`crates/viewer-ui/src/bin/pdf-viewer/cadence.rs`. One period, one instant, one flag:

- **the period is the surface's**, from `winit`'s `MonitorHandle::refresh_rate_millihertz`, read in
  `resumed` because that is the first moment there is a surface to ask. `doc/todo/36`'s **floor of
  60 Hz** stands in where the platform states none, and the trace says which of the two this run
  got — "we present at 60 Hz" and "this display refreshes at 60 Hz" are different claims;
- **`due` is a rate limit rather than a timer.** `redraw_requested` asks whether the surface has
  refreshed since the last present; if it has not, the frame is *deferred* to the tick rather than
  drawn. A window that has been still is due at once, so **input latency is unchanged**: the
  cadence is a ceiling on how often a frame is drawn and never a floor under how soon one is;
- **`owing` is what the loop waits on.** `about_to_wait` chooses `ControlFlow::WaitUntil(next
  tick)` when a frame is owed and `ControlFlow::Wait` when none is.

**What that changed, and it is more than the reprojection path.** Three call sites moved:

| | before | after |
|---|---|---|
| a redraw asked for by an input | drawn at once, however many arrived per refresh | drawn on the next tick |
| the frame replacing a reprojection | `request_redraw` inside `MustFollow::follow` | the clock, so a view still moving is answered again first |
| §12.4.4's transition in flight | `ControlFlow::Poll` — a core at 100% for its length | one frame per refresh |

`MustFollow` is unchanged as a type and still cannot be dropped; what it discharges its obligation
*to* is the clock rather than the window. Rule 1 gained an enforcement rather than losing one:
`about_to_wait` still refuses to leave the loop at rest with a reprojection on the screen.

### 2. Compose, do not chain — and it is the type that guarantees it

The owner permits a reprojection of a reprojection. Resampling a resampling compounds the blur, so
none is ever taken:

- `Stale::Settled` — the last **real** frame — gained a `base`: that frame's own pixels, read back
  once, held as the `Arc<[u8]>` `render-quorra`'s resource cache keys an upload by;
- `Stale::plan` composes `settled.transform⁻¹ ∘ asked.transform` **always against the rendering**,
  whatever is on the screen. Ten reprojections in a row are ten single resamples of true pixels;
- the capture is asked for only when `wants_base()` — that is, only when the window is showing a
  rendering — so **there is no state in which a capture could be of a reprojection**;
- the pixels leave the module only through `Stale::reproject`, a method rather than a free function
  taking a raster, so no caller can hand in something else to resample.

That is the property as a type rather than as a rule somebody follows, which is what `doc/todo/37`
rule 2's structural argument asks of everything in this file.

### 3. A late frame re-bases

`Stale::settled` replaces the whole `Settled`, base included. A delayed frame that lands while the
view has moved on becomes the base at once and the composed transform simply changes. One field,
one invariant, and nothing to remember: the pixels and the placement they were drawn at are the
same fact, so they are replaced together.

### 4. Rule 5's threshold is left exactly as ADR 0378 wrote it

`SHARE × this run's worst reprojection`. It is now measuring a **bimodal** population — the first
reprojection of a base pays the readback and the rest do not — so the worst-case bound overstates
the common case by an order of magnitude (below). Changing it would be inventing policy inside a
round that was asked to build a cadence, and the direction of the error is the safe one: it
approximates *less* often than a cost-of-the-common-case bound would. Written down rather than
taken; a round that wants it is welcome to the argument.

## The measurement

`Xvfb :NNN` at 900×1100, llvmpipe, the release binary of this tree, `tmp/Entwurf.pdf` (49 MB, one
page, 58 009 display commands — not in the repository and named in no test). Driven by `xdotool`.
**The surface states no refresh rate** — `xrandr` on Xvfb reports `0.00` and `--newmode` does not
take — so every run below asks for the 60 Hz floor, and the trace says so on its own.

### The rate: 200 `+` keys at 8 ms, which is input at 120 Hz

```text
97 present(s): 87 a rendering of the page and 10 a reprojection of one (89.7% correct)
intervals between presents, ms: median 16.7  p90 16.9  p99 3660.7  max 3660.7
                               — 94 of 96 on the next refresh (97.9%)
```

The p99 and the max are the one gap between the launch frame and the first key; the driven stretch
alone, off the trace's own clock, is 95 intervals:

```text
min 8.0   p10 16.0   median 17.0   p90 17.0   p99 73.0   max 73.0
  [  8,  12) ms     1
  [ 12,  14) ms     1
  [ 14,  16) ms     3
  [ 16,  18) ms    89   ############################################################
  [ 50,  80) ms     1   ← the correct frame itself, four refreshes long
```

**Eighty-nine of ninety-five intervals are one refresh**, against a 16.667 ms period asked for. The
tail is one 73 ms interval and it is the honest limit named below.

### The correct-frame fraction, which is a different claim

**87 of 97 presents were a rendering of the page** and 10 were a reprojection of one. Both numbers
are in the trace's own summary, which is where rule 6 asks for them.

### What a reprojection costs now, and why the cadence is affordable at all

The ten in one burst, in order, off the same base:

| | whole reprojection | readback | uploads |
|---|---:|---:|---:|
| the first, standing in for a 656.4 ms frame | **45.1 ms** | 31.2 | 1 |
| the nine after it | **3.7 – 5.3 ms**, median 3.9 | none | **0** |

**A factor of 11.6 between the first and the rest**, and it is the whole design: ADR 0378 paid the
readback on *every* reprojection, which at 19–36 ms is more than a refresh at either rate. Paying
it once per real frame and resampling the base under a recomposed transform is what puts a
reprojection inside a tick — 3.9 ms fits a 120 Hz refresh (8.33 ms) with headroom, and fits a
60 Hz one four times over.

The correct frame that followed them cost 60.0 ms and re-based; the replays after it, 1.4 ms
apiece, are what the clock then spaced at 16.7 ms.

### What an idle window costs

Twenty seconds with the document open and no input at all, on the witness:

```text
IDLE_BEGIN frames=1 cpu_ticks=285
IDLE_END   frames=1 cpu_ticks=285
```

**No present, and no measurable processor time** — zero ticks at 100 Hz `USER_HZ`, so under 10 ms
of CPU in 20 s. The loop is in `ControlFlow::Wait`; the clock is armed by an obligation and by
nothing else. `doc/todo/36`'s fourth rule, measured rather than claimed.

### The A/B that says it is the clock doing this

The same script against the same binary with only the redraw gate taken off — the state this round
passed through, when the clock paced the frames after a reprojection and nothing else:

```text
78 present(s) … intervals between presents, ms: median 8.3  p90 8.6
```

**The window ran at the input's rate**, which is what `doc/todo/36` calls a stutter machine and is
exactly why it says the cadence must be the surface's. With the gate, the same 8 ms input produces
16.7 ms presents. It is also the strongest evidence this harness can give about the target:
**this presenter does sustain 8.33 ms intervals** — that run is 78 presents at a median of 8.3 —
for frames that fit, and a replay (1.3–1.5 ms) and a composed reprojection (3.4–5.9 ms) both do.

### So: is 120 Hz reached?

Stated as three claims, because they have three different answers:

- **The cadence asked for** is the surface's, and on this harness the surface states none, so
  60 Hz stands in. **120 Hz cannot be observed here at all** — Xvfb has no refresh rate to report
  and `xrandr` will not give it one. The path that reads it is a unit test and nothing more.
- **The cadence achieved** at the rate asked for is 16.7 ms median, 17.0 at p90, 91 of 95
  intervals inside one refresh. At 60 Hz, on a software adapter, **the presenter holds the
  cadence.**
- **Whether a correct frame holds it** is a question about the renderer and the answer is no on
  this adapter: a correct frame of this page costs 55–73 ms, which is four refreshes at 60 Hz and
  eight at 120. That is what the reprojection is for and what every round that makes a frame
  cheaper takes away — `doc/todo/37`'s standing intent, and worth saying that it has moved a long
  way already: ADR 0378 measured this witness's zoom step at 492–1036 ms and it is now 55–73.

**So the answer with its number is: 60 Hz is reached and 120 Hz is not established here.** The
presenter is not what stops it — it held 8.3 ms intervals for 78 presents in the A/B above — and
neither is the reprojection, at 3.9 ms. What stops it on this machine is a correct frame of this
page on a software adapter, and what stops the *measurement* is a virtual display with no refresh
rate to state.

## What a clock cannot do here, and it is the honest limit

**`QuorraPresenter::present` runs to completion on the event thread.** So the clock decides when a
frame may *start* and has no say in how long one lasts: the 79 ms interval in the histogram is the
65.7 ms correct frame plus its tick, five refreshes during which nothing could be presented. The
owner's *"we should still try to render a correct image every frame"* is honoured in the only sense
a synchronous renderer permits — a correct frame is attempted at every tick at which no reprojection
is owed — and it cannot be honoured in the pipelined sense, where a presenter presents at the
deadline whatever a renderer running beside it has finished.

Removing it means taking the render off the presenter's thread, and that is **not** a small change
disguised as a large one: `quorra_gpu::Device::render` takes `&mut self` and carries the caches, so
one device cannot serve a render thread and a present thread at once. It is a separate item with
its own argument, and it is written down as one (`doc/todo/36`, amended) rather than smuggled into
this round.

## ADR 0382 §6 is corrected: the texture is not enough, and the seam is the *surface*

That ADR concluded that rendering into a `Target::Texture` the presenter owns answers this round's
need with "no readback and no upstream change needed", and that "the escape hatch is complete".
**The first half is right and the second is not**, and the difference is worth writing down because
the next round will otherwise start where 547 left off:

- a host *can* have quorra render into a texture it owns and sample it — `validate_texture` asks
  `usage().contains(RENDER_ATTACHMENT)`, `Device::wgpu()` hands over the same device and queue, and
  `quorra-gpu` re-exports `wgpu`. That much was checked and holds;
- but **presenting that texture requires the surface, and quorra owns it**. `quorra_gpu::Device`
  keeps `surface: Option<SurfaceState>` private and exposes no accessor: `description`, `wgpu`,
  `limits`, `coverage`, `set_coverage` and the constructors are the whole of its public surface;
- so a host would have to create a `wgpu::Surface` of its own and configure it — and
  `Surface::configure` needs a format, while both ways of *learning* a supported format,
  `get_capabilities` and `get_default_config`, take a `&wgpu::Adapter` (wgpu 30.0.0
  `src/api/surface.rs`). **`quorra_gpu::Device` returns no adapter.** A host that guessed the
  format would be guessing about somebody else's driver, and this project does not put a guess
  between a page and a screen.

So `doc/QUORRA_FEEDBACK.md` §28.6's question stands, and it is sharper than it was: the ask is
either `Device::present_texture(&wgpu::Texture, Affine, into: Target)` — the thing §28.6 already
described — or, much smaller, an accessor for the adapter or the negotiated surface format, which
would let a tier-3 host be written here without a guess. Either answers it; neither is needed for
what this round built, which is why nothing waited on it.

**And this round did not need a texture at all.** The reason is arithmetic rather than architecture:
a readback costs 25 ms and a tick is 8 to 17, so a readback per *reprojection* is impossible and a
readback per *real frame* is free — the real frame it follows costs 55 to 700. Amortisation was the
cheaper answer than a new present path, and it needs nothing from anybody.

## What this costs

- **One readback per real frame that is ever reprojected**, and none for a frame that is not: the
  capture is lazy, so a window nobody is touching reads nothing back. 25.3 ms on this adapter,
  against the 65–700 ms frame it follows.
- **The base's samples are held**: one window of RGBA8, 3.2 MB at 800×1000, released when the next
  real frame replaces the `Settled` that owns it.
- **One extra scene rebuild per reprojection**, unchanged from ADR 0378, and now visible as the
  `settle` column of 0.5–1.0 ms on the reprojection lines themselves.
- **A frame may be up to one refresh late** where a redraw arrives just after a tick. That is the
  price of not presenting faster than the display, it is bounded by the period, and the first frame
  after a still window pays none of it.

## Clauses

**None**, and for ADR 0378's reason unchanged: this is presentation and not a reading. Nothing
reprojected is a *rendering* of the page, so §10.7.4's scan-conversion rule does not reach it, and
the conformance ledger is unmoved. §12.4.4's transition changed only in how often its frames are
drawn, which the clause does not state.

## What did not move

`fmt`, `clippy --workspace --all-targets`, the workspace test run, the doctests, the conformance
checker, the corpus gate, the oracle, both text gates, and both of `render-quorra`'s coverage
lanes — which is `doc/todo/37` rule 2's own gate, since nothing that judges a picture may ever see
one of these frames and an instrument that had started photographing one would show up there and
nowhere else. The session's own file carries the figures. **No library changed**: everything this
round wrote is in `crates/viewer-ui/src/bin/pdf-viewer/`, a binary, and the test that walks every
`.rs` outside it still passes.
