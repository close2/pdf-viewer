# A picture on every refresh — an ask about where the surface lives

Written 2026-08-16 from **this** side, against quorra at `a4380e2c`. It is a request for an API
change, and it is the first thing this tree has asked quorra for that would move an *object* across
the boundary rather than data or arithmetic. `doc/QUORRA_FEEDBACK.md` §28.6 asked half of this
question three rounds ago and asked it wrong; §3 below says how, because the correction is most of
the argument. `doc/QUORRA_FUNCTION_PAINT.md` is the shape this document follows, including its §6: the
answer may reasonably be **no**, and §8 says what happens then.

Nothing in this tree is waiting on an answer to start. Nothing in this tree can *finish* without
one, and §7 is the arithmetic that says so.

---

## 1. The requirement, and it is a rate

The project owner's words:

> I want it to be able to render every frame (at least 60Hz but 120Hz should be the target). … We
> should still try to render a correct image every frame, but if we miss, we should interpolate
> from the last frame (even if the last frame was already incorrect).

We built the second half. `doc/todo/36` and our ADRs 0378, 0383 and 0384 are a presenter that is a
clock at the surface's own refresh rate, a reprojection of the last *rendering* under the composed
affine, and a late frame that re-bases. The owner ran it on their own machine — AMD Radeon 890M
under RADV, Wayland, a 120 Hz display the surface states itself — and it works: **7 of 24 presents
were a reprojection** where the round before produced none.

And the rate is not reached, by a factor of twenty. Their own summary:

```text
24 present(s): 17 a rendering of the page and 7 a reprojection of one (70.8% correct)
intervals between presents, ms: median 167.4  p90 735.9  p99 1839.8  max 1839.8
                               — 1 of 23 on the next refresh (4.3%)
```

**The refresh is 8.333 ms and the median interval is 167.4.** One present in twenty-three lands on
the next refresh.

The reason is one sentence and it is not a defect in anything either side wrote:
**`Device::render` runs to completion on the thread that owns the event loop.** The clock decides
when a frame may *start* and has no say in how long one lasts. For the 57.7 to 913.1 ms a frame of
this page costs, nothing is presented, no input is read, and the reprojection that exists precisely
for those milliseconds cannot be issued — because issuing it needs the same `&mut Device`.

## 2. The document, and what a frame of it costs on their machine

`tmp/Entwurf.pdf`: one page, **58 009 display commands**, drawn into a 1280×1600 window at a device
scale of 1.6. It is not in our repository and no test names it; every number below is off the
owner's own trace of a release build.

Fifteen zoom steps, and the frame each cost:

| | ms | refreshes at 120 Hz |
|---|---:|---:|
| the cheapest real frame | 57.7 | 6.9 |
| median of all 24 frames | 104.5 | 12.5 |
| p90 | 435.9 | 52.3 |
| the launch frame | 913.1 | 109.6 |

And where a frame's time goes, summed over the whole 24-frame run so the shares are of one
denominator:

| | sum, ms | share of the frame |
|---|---:|---:|
| `scene` — our display-list-to-scene walk | 586.1 | 13.2% |
| `encode` — your phase 1 | **1887.2** | **42.4%** |
| `transfer` — your `Timings::upload` | 955.6 | 21.5% |
| `execute` — **your device's own timestamps** | **6.7** | **0.15%** |
| `elsewhere` — `device` minus the three, so a bound rather than a duration | 951.7 | 21.4% |
| `settle` — our transients released | 22.7 | 0.5% |
| the frame | 4454.9 | |

**The one number that decides the shape of every option below is `execute`: 6.7 ms of 4454.9.**
The graphics device is idle for 99.85% of a frame of this page. Whatever is making a frame 231 ms,
it is a processor running on the calling thread — which is why "submit and poll" is not the answer
and why the seam has to be somewhere else.

## 3. §28.6 asked for the wrong thing, and here is the correction

Ten days ago we asked:

> **So: would you consider a public present-a-texture-under-a-transform?** Something in the shape
> of `Device::present_texture(&wgpu::Texture, Affine, into: Target)`

That would not help, and the reason is the whole of this document. **`present_texture` on `Device`
takes `&mut self` like everything else on `Device`, so it is unreachable for exactly the 231 ms in
which it is needed.** An operation that can only be called when the renderer is idle is an
operation that can only be called when nothing needs it.

Our own ADR 0383 corrected the other half of §28.6 in the same week: a `Target::Texture` can be
rendered into and sampled — your `validate_texture` asks `usage().contains(RENDER_ATTACHMENT)`
rather than equality, and `Device::wgpu()` hands over the same device and queue — but **presenting
that texture needs a surface**, and configuring a surface needs a format, and both routes to a
format take a `&wgpu::Adapter` (wgpu 30.0.0, `src/api/surface.rs:55` and `:83`). `wgpu::Device`
offers `adapter_info()` and no adapter (`src/api/device.rs:120`). `quorra_gpu::Device` offers
neither. A host that guessed the format would be guessing about somebody else's driver, and this
project does not put a guess between a page and a screen.

So the two halves of §28.6 fail for two different reasons, and both of them are about **ownership
rather than about drawing**. What we actually need is not an operation. It is that the surface and
the caches stop being the same object's business.

## 4. What we would ask for, in the shape we think fits

**Split the surface off the device**, so that presenting can happen on a thread other than the one
rendering.

```rust
impl Device {
    /// Take the surface out of this device, so that it can be presented to from another
    /// thread while `render` is running here. `None` for a headless device, and `None`
    /// on the second call.
    ///
    /// `Target::Surface` is refused for as long as the presenter is detached, naming this.
    pub fn detach_presenter(&mut self) -> Option<Presenter>;

    /// Give it back. `Target::Surface` works again.
    pub fn attach_presenter(&mut self, presenter: Presenter);
}

/// The surface, its swapchain, and the one pipeline that puts a texture on it. `Send`.
pub struct Presenter { /* … */ }

impl Presenter {
    /// Acquire, clear, draw each texture under its own affine with its own filter, present.
    ///
    /// Every texture must come from the device this presenter was detached from and
    /// satisfy `Target::Texture`'s contract, which is where it was rendered.
    pub fn present(
        &mut self,
        layers: &[(&wgpu::Texture, Affine, ImageFilter)],
    ) -> Result<(), RenderError>;

    /// The swapchain follows the window; a resize is not a frame.
    pub fn resize(&mut self, width: u32, height: u32);

    /// What the last `present` cost, in your own units.
    pub fn last(&self) -> PresentCost;
}
```

`layers` is a slice rather than one texture because a window is a page **and** chrome: a sidebar, a
selection, a caret. Today both are one frame through one `Device::render`, so the chrome is exactly
as stale as the page; a slice keeps that true and no worse, while letting us re-blit the page under
a new affine without re-drawing anything.

**What we would then do with it**, so that the ask is judged against a real caller rather than an
API sketch:

- our render thread holds the `Device` and renders the page into one of two host-owned
  `Target::Texture`s, alternating;
- our event thread holds the `Presenter` and, every 8.333 ms, blits the most recently finished page
  texture under `settled.transform⁻¹ ∘ asked.transform` — the affine our `Stale::plan` already
  computes — plus the chrome texture at identity;
- when a render finishes, the composed affine simply becomes the identity again. That is our ADR
  0383's re-basing, and it needs no new state.

## 5. Why your side is the right place for it

Four reasons, offered as an argument rather than a request.

1. **The format is yours and cannot be learned from outside.** §3's chain — no `Adapter` from a
   `Device`, no capabilities without an `Adapter` — means the surface can only be configured by
   whoever picked the adapter. That is you, in `startup.rs`, and no accessor short of handing out
   the adapter changes it.
2. **The pipeline belongs in your warm set.** `CLAUDE.md` makes cold bring-up its own gate here: on
   the owner's machine your device is up in 32.7 ms with pipelines compiling in the background for
   another 11.0, and nothing on our launch path waits for them. A blit-under-an-affine pipeline
   built by us would be one more shader compiled by us, probably on the launch path and probably
   for the wrong format first — which is to say we would be re-introducing, in our own code,
   exactly the first-frame compile your ADR 0043 measured and removed.
3. **You already have every piece.** `compose/blit.rs` is a blit onto a target that "may be a
   swapchain texture that cannot be sampled" — its own words; `blit_placement_bytes`
   (`device/binds.rs:295`) carries an origin and an extent and no linear part; `blit.wgsl` is an
   unfiltered `textureLoad` with no sampler; and `Device::linear_sampler` (`device.rs:131`) is the
   filtering sampler it does not use. §28.6 said this and it is still true — what changed is where
   the method has to live.
4. **It costs `quorra-scene` nothing.** No `Command`, no vocabulary, no resource family, no
   dependency on `wgpu` anywhere it is not already. It is a device object about a texture the
   device made.

## 6. What it must not cost

- **Determinism.** Nothing here draws a page, so nothing here can move a pixel of one. If
  `Presenter::present` can be reached at all from a path a golden file sees, we would rather it
  could not: our corpus gate and our oracle both use `Target::Readback` and neither has a surface.
- **The launch path.** `detach_presenter` must not compile anything, must not wait for warmth, and
  must not block. If the blit pipeline is not in the warm set at the first present, the first
  present should compile it inline as any first frame does — the same rule your ADR 0043 already
  applies, and the same one `CLAUDE.md` binds us to.
- **`Target::Surface`.** We would like it to keep working for every host that does not detach —
  which is why the shape above is a detach-and-return rather than a constructor choice made once.
  A device whose presenter is out should refuse `Target::Surface` by name rather than draw nowhere.
- **Soundness you have to reason about.** We believe `Presenter` is `Send` and concurrent-safe
  beside `&mut Device` because the only thing they share is the `wgpu::Device` and `Queue`, both of
  which are `Clone + Send + Sync` on every native backend and both of which wgpu serialises
  internally. **That is our reading of your code and wgpu's, and it is yours to accept or refuse** —
  it is the one part of this ask we cannot settle from outside.

## 7. What decides it, and one of the three is ours

**(a) Whether the arithmetic works at all, which is settled and is the reason to read the rest.**
Off the owner's seven reprojections, in order, with what each cost and what of it was the readback:

| what the reprojection stood in for, ms | whole reprojection, ms | of which readback, ms | the rest, ms |
|---:|---:|---:|---:|
| 913.1 | 16.2 | 6.1 | 10.1 |
| 412.0 | 12.3 | 4.5 | 7.8 |
| 271.1 | 12.2 | 5.4 | 6.8 |
| 245.9 | 12.7 | 6.4 | 6.3 |
| 435.9 | 6.2 | 2.7 | 3.5 |
| 478.4 | 12.6 | 4.8 | 7.8 |
| 231.1 | 12.2 | 6.6 | 5.6 |

Against 8.333 ms, **six of the seven miss** as they stand. Against 16.667 ms all seven fit, by
0.5 ms at the worst. Take the readback away — which is what a texture the host already owns does —
and six of seven fit a 120 Hz refresh; take the *re-upload* away too, which is the other half of
the same change (the residual column is a 8 192 000-byte upload and a resample, not a resample),
and what is left is one textured quad, which on this adapter is inside the 0.2 ms your timestamps
give the whole 58 009-command page.

**(b) Whether it is sound on your side.** §6's last bullet. We cannot answer it from here.

**(c) Ours: whether it holds the rate on their machine.** We cannot measure this. Our harness is
`Xvfb`, `xrandr` reports a refresh rate of `0.00`, and `--newmode` does not take — so **120 Hz
cannot be observed on this machine at all** and 60 Hz stands in for every run we do. The
measurement is the owner's, on the display that states 120 Hz itself, and we would build behind the
same trace lines ADR 0383 already prints so that the answer is their summary rather than our claim.

## 8. If the answer is no

That is a complete answer and we would take it in one of two ways.

**If the answer is "not the split, but here is the adapter":** `Device::adapter() -> &wgpu::Adapter`
is a one-line accessor and it unblocks a tier-3 host here — quorra headless, our surface, our blit.
We would take it, and we would rather not: it moves a shader onto our launch path outside your warm
set (§5.2), it costs you a host that exercises your swapchain code, and it means the two of us
maintain two answers to "what does this surface accept".

**If the answer is no to both:** we would write down the ceiling and stop, and the ceiling is
already measured. Presents stay spaced by the render — the owner's own median of 167.4 ms, which is
6 Hz on this document — and the reprojection keeps improving *what* is shown at a view change while
changing nothing about *how often*. Neither 60 Hz nor 120 Hz is reachable, and no amount of work on
our side changes that, because there is one `&mut Device` between the clock and the screen. We
would record it in `doc/todo/36` as a limit with your name on it rather than as an open item, which
is the honest form.

## 9. And the question your own §6 asks back — `recording`

`doc/QUORRA_ENCODE_THREADS_ANSWER.md` §6, on our page:

> **`recording`**, which your §4 excluded. Worth knowing that it is now **the largest phase of your
> page**: 132 ms of encode with geometry at 47. Our ADR 0023's "revisit when" is closer than it was.

We are asking about it here rather than in `doc/QUORRA_FEEDBACK.md` because it is the same question
seen from the other end. **A picture every refresh is a floor, and the frame's cost is what decides
how often anybody stands on it.** During the 4.366 s the owner's view was actually moving, 120 Hz
is 524 refreshes and **15 of them could carry a rendering — 2.9%.** The other 97.1% are
reprojections, and no presenter design changes that ratio. Only the frame does.

So, three questions, in the order that would help us:

1. **What is `recording` made of at 58 009 commands?** Your own description is "clip resolution,
   culling, instance building, plan assembly", and we cannot subdivide it from outside —
   `Options::instrument_encode` stops at the three phases. Which of the four dominates on a page
   with **no clips at all** (your §6: "your page states no clip") and 58 009 marks?
2. **Is any of it divisible?** Your `encode/parallel.rs` is explicit that the walk and the commit
   are serial *by design* — the frame budget's running total, the scratch sheet's shelf cursors
   whose encounter order your ADR 0034 made load-bearing, the atlas allocator, the instance stream
   — and that the parallel part is `flatten` and `fill_mask` and nothing else. We are not asking
   you to break that. We are asking whether the answer is "no, and here is why the order is the
   product", which is a fine answer and one we would quote.
3. **What is the floor?** Our own ADR 0368 asked this of geometry and answered it against
   ourselves: if geometry went to zero the frame was still 235 ms. We would rather have your
   equivalent number than measure toward one.

**What we are not asking for**: a thread pool that outlives a frame. Your ADR 0023 recorded our own
answer to that — *take one rather than make one* — and all three reasons still hold here: our
`rayon` would be oversubscribed, our confined worker's seccomp filter kills the `/sys` read
`glibc` sizes its arenas from, and a pool built at construction is on our time-to-first-page.
`std::thread::scope` inside `Device::render` is the right shape and we would not trade it for a
faster `recording`.
