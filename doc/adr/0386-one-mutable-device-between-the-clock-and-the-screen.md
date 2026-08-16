# 0386 — One `&mut Device` between the clock and the screen

**Status.** Accepted. Session 551. A **design** round: it changes no code and ends in this document
and in `doc/QUORRA_NONBLOCKING_RENDER.md`. Closes the open section of `doc/todo/36` with an answer
rather than with a build. Rests on ADR 0383 (the presenter as a clock, and the base it composes
against), ADR 0384 (the two thresholds re-grounded on the owner's own hardware), ADR 0368 (where a
frame's time goes) and ADR 0382 §6 as corrected by ADR 0383.

## The question

The project owner's, in one sentence:

> *will we be able to achieve either correct or reprojected frames (for every frame)?*

with 60 Hz as the floor and 120 Hz as the target, and their own machine — AMD Radeon 890M under
RADV, Wayland, a display that states 120 Hz — as the place it has to be true.

The answer this round arrives at, before the argument: **yes at both rates, and only with a change
on quorra's side.** Everything this tree can do alone has been done, and what is left is one
sentence of ownership.

## 1. The arithmetic, per rate, off the owner's own trace

`tmp/Entwurf.pdf` in a 1280×1600 window at device scale 1.6, 58 009 display commands, the release
binary of the tree at session 549, driven by hand. Not in the repository and named in no test;
`tmp/trace3.entwurf.txt` is the run.

### What a picture on every refresh requires

Three things have to hold, and this file states the third because nobody had:

1. **a path to the screen that does not wait for the render.** Real frames of this page cost 57.7
   to 913.1 ms — 7 to 110 refreshes at 120 Hz, 3.5 to 55 at 60. Nothing else on this list matters
   until this one holds;
2. **a reprojection that fits the period**;
3. **a base for it to compose against**, on every view change. The owner's run printed the refusal
   **twice** against its seven reprojections — `capture_presented` returns `Ok(None)` when the last
   frame repacked the glyph atlas, because the retained encode died with the tile placements (ADR
   0384 §6). So even with 1 and 2 held, some view changes have nothing to show, and the readback
   route is why.

### What a reprojection costs there, and the split that decides the rate

The seven, in order, from the trace's own `approximated:` lines:

| stood in for a frame of, ms | whole reprojection, ms | of which readback, ms | the rest, ms |
|---:|---:|---:|---:|
| 913.1 | 16.2 | 6.1 | 10.1 |
| 412.0 | 12.3 | 4.5 | 7.8 |
| 271.1 | 12.2 | 5.4 | 6.8 |
| 245.9 | 12.7 | 6.4 | 6.3 |
| 435.9 | 6.2 | 2.7 | 3.5 |
| 478.4 | 12.6 | 4.8 | 7.8 |
| 231.1 | 12.2 | 6.6 | 5.6 |

**Every one of the seven paid a readback**, because on that machine a real frame lands between any
two view changes and re-bases. So the "steady state" figure — a second reprojection off a base
already captured — has **never been measured on the owner's hardware at all**. The right-hand
column is a subtraction, and this file says so rather than quoting 5.6 ms as a measurement. What it
is a measurement of, on the llvmpipe harness, is ADR 0383's table: 45.1 ms for the first and
3.7–5.3 for the nine after it.

Note also what the right-hand column still contains: **an 8 192 000-byte upload**. Every one of
those seven frame lines reads `1 up` — the captured window is handed back to the device as a raster
before it can be resampled. So "take the readback away" and "take the re-upload away" are two
halves of one change, not two changes.

### At 60 Hz — 16.667 ms

- reprojection: **all seven fit**, worst 16.2 ms, margin 0.5 ms. It fits by 2.8% at the tail, which
  is a fit and not a comfort;
- so 60 Hz needs **requirement 1 and nothing else about cost**. The picture is affordable already.

### At 120 Hz — 8.333 ms

- reprojection as it stands: **six of seven miss.** Only the 6.2 ms one fits;
- reprojection without the readback: **six of seven fit**, the 10.1 ms one misses;
- reprojection without the readback *and* without the re-upload — one textured quad against a page
  whose whole 58 009 commands `execute` in 0.2 ms: not measurable at this resolution;
- so 120 Hz needs requirement 1 **and** `doc/QUORRA_FEEDBACK.md` §28.6's no-readback path. That
  asymmetry is the thing the owner had not been told in one place, and it is the reason this file
  exists.

### And the ceiling on the *correct* frame, so that nobody reads the above as a promise

The cheapest real frame of this page on their machine is **57.7 ms**, the median 104.5. A correct
frame every refresh at 120 Hz asks for 12.5× on the median and 7× on the best, and ADR 0368
measured that the parts of a frame which are not geometry do not divide. **During the 4.366 s the
owner's view was moving, 120 Hz is 524 refreshes and 15 of them could carry a rendering — 2.9%.**
The reprojection is a floor under the experience and it is *most* of the experience on this
document. Everything that makes a frame cheaper reduces how often it is seen, which is
`doc/todo/37`'s standing intent, and none of it changes the answer to the owner's question.

## 2. The obstacle, verified at the pinned revision rather than quoted

`doc/todo/36` named it: `quorra_gpu::Device::render` takes `&mut self` and owns the caches and the
surface. Checked against quorra at `a4380e2c`, which is what `Cargo.lock` pins:

- `Device::render(&mut self, …)` and `Device::render_retained(&mut self, …)`
  (`quorra-gpu/src/device/render.rs:50`, `:105`);
- the struct holds `surface: Option<SurfaceState>` beside `resources`, `atlas`, `atlas_texture`,
  three texture maps and the winding target (`device.rs:107`–`159`);
- twenty-seven public methods, and **not one of them returns an adapter or the surface**. The
  accessors are `description`, `wgpu`, `limits`, `coverage`, `startup`, `resource_bytes_in_use`,
  `is_warm` and `warm_up`; `invalidate_surface` is the only public reach into the surface at all
  and it forces a reconfigure rather than handing anything over. **And there is no constructor that
  adopts an existing `wgpu::Device`** — `headless`, `headless_with_instance`, `for_surface` and
  `for_surface_with_instance` each select an adapter and create a device of their own.

Two consequences follow immediately and are used below. A **second `quorra_gpu::Device` is a second
`wgpu::Device`**, so its textures are not the first one's. And **a host cannot configure a surface
of its own**: `Surface::get_capabilities` and `Surface::get_default_config` both take
`&wgpu::Adapter` (wgpu 30.0.0, `src/api/surface.rs:55`, `:83`), `wgpu::Device` offers only
`adapter_info()` (`src/api/device.rs:120`), and quorra offers no adapter. ADR 0383 read this three
sessions ago and it is re-read here at the pinned revision rather than carried forward.

**And the third fact is the one that reframes the whole question**: over the owner's 24-frame run,
`execute` — the device's own timestamps over the drawing passes — is **6.7 ms of 4454.9**, which is
0.150%. `encode` is 42.4%, `transfer` 21.5%, `elsewhere` 21.4%, our `scene` 13.2%. The graphics
device is idle for essentially the whole of a frame of this page. A frame here is a *processor*
running on the calling thread, and any option whose mechanism is "stop waiting for the GPU" is
bounded above by 0.15% before it is priced.

## 3. The options, each priced

| | what it costs here | what it costs quorra | what it buys | verdict |
|---|---|---|---|---|
| **(a)** a non-blocking `render` upstream — submit, then poll | a poll loop; a `Frame` that outlives the call | a split API, and `&mut self` still held across the split | **the device's own time: 6.7 ms of 4454.9, 0.15%.** `execute` is 0.2 ms at the median | declined on the measurement |
| **(b)** the encode on quorra's own pool, caches synchronised internally | nothing | a pool outliving a frame — which contradicts *our* answer in their ADR 0023 — plus shared mutable atlas, scratch sheet and instance stream whose encounter order their ADR 0034 made load-bearing, plus determinism across thread counts | a **cheaper** frame, never an asynchronous one. Subtracting the `encode` column from the frame column leaves ≈26 ms at the median and ≈260 at p90 — still 3 and 31 refreshes | keep asking; it does not answer this question |
| **(c)** a render thread here with a **second device** | a second adapter, a second device, a duplicate atlas and duplicate resident resources | nothing | nothing that pays for itself | **priced and declined**, §3.1 |
| **(c′)** a render thread here with the **same** device, split at the surface | a render thread, a texture pair, `QuorraPresenter` in two halves | one object moved out of `Device` | **the only option that reaches either rate**, §3.2 | **recommended** |
| **(d)** chunked/resumable encode | a frame driven to completion across ticks | half a frame's `&mut self` state alive between calls, plus a rollback for an abandoned one | nothing without (c′), and 28 re-entries with it | strictly dominated |
| **(e1)** tier 3 with the host owning the surface | our own blit pipeline on our launch path, outside quorra's warm set | nothing | would work — if the format could be learned | blocked at §2's format chain |
| **(e2)** make the frame fit the refresh | — | — | 12.5× on the median, 7× on the best | not reachable; ADR 0368 |
| **(e3)** the page-space scene under a root affine | `Encoder` stops composing the placement; every overlay pre-transformed | nothing | **2.4%** of a zoom frame (ADR 0368) | already declined, unchanged |
| **(e5)** the scene build off-thread alone | a thread for `scene` | nothing | 13.0 ms of 104.5 — 12.4%, and still more than a refresh | insufficient alone |
| **(e6)** defer the real frame until the view is at rest | a policy change in `stale.rs` | nothing | **60 Hz during a gesture, today, with no ask** — and no correct frame during one | §3.3; available, and it trades the owner's own sentence |

### 3.1 The second device, priced rather than assumed

`doc/todo/36` said a second device was one of three things whoever took this item might argue for.
It is the cheapest to dismiss once the price is written down:

- **the page would cross as bytes.** wgpu 30 exposes no cross-device texture sharing —
  `create_texture_from_webgpu_handle` is the WebGPU backend's and `create_external_texture` is for
  video — so the frame would be read back off device A and uploaded to device B. At 1280×1600 RGBA8
  that is **8 192 000 bytes per frame**, at the owner's own measured readback rate of 2.7–6.6 ms
  plus an upload of the same size: **more than a 120 Hz refresh before anything is drawn**, and
  most of a 60 Hz one;
- **a second bring-up.** Their trace: adapter selection 15.6 ms, device creation 12.6 ms, and a
  second pipeline set to compile;
- **duplicate residency.** The busiest frame's glyph tiles want 3 570 512 bytes and the retained
  encode holds up to 7 141 247; both would exist twice.

It buys exactly what (c′) buys and pays 8.2 MB a frame for it. Declined.

### 3.2 The recommendation: one device, two threads, split at the surface

The shape, and the reason it works where (a) and (c) do not:

- **the render thread holds the `Device`** and renders the page into one of two host-owned
  `Target::Texture`s, alternating. `validate_texture` asks `usage().contains(RENDER_ATTACHMENT)`,
  so a texture created `RENDER_ATTACHMENT | TEXTURE_BINDING` is accepted and can be sampled
  afterwards (`device/bound.rs:125`);
- **the event thread holds a presenting handle** — quorra's `Presenter` in the ask — which owns the
  surface and blits the most recently finished texture under the affine `Stale::plan` already
  computes. No `&mut Device`, no readback, no upload, no encode;
- **they share one `wgpu::Device` and one `Queue`**, which are `Clone + Send + Sync` on every native
  backend and which wgpu serialises internally. §2's `execute` number is why the contention does
  not matter: the queue is idle 99.85% of the time.

**What it buys, itemised against §1's three requirements:**

1. the render no longer stands between the clock and the screen — requirement 1, which is the whole
   of what 60 Hz needs;
2. the base is a texture rather than a readback, so requirement 2 is met at 120 Hz as well;
3. **and requirement 3 falls out for free**, which nobody expected. `capture_presented` refuses
   after an atlas repack because the *retained encode* died — but a texture quorra has already
   rendered into is unaffected by a repack. The owner's two printed refusals stop being refusals,
   and ADR 0384 §6's "it cannot be worked around from here" becomes true only of the readback
   route.

**What it does not buy**: a correct frame every refresh. §1's ceiling stands, 2.9% of them.

### 3.3 The one thing available without an ask, and what it trades

**(e6) is real and this round did not take it.** During a gesture, nothing forces the real frame to
be asked for at once. `Stale` already composes every reprojection against the last *rendering*, so
a chain of them compounds no blur (ADR 0383 §2), and the first pays the readback while the rest do
not. A presenter that deferred the real frame until the view came to rest would present a
reprojection every 16.667 ms through the whole gesture, on this side alone, with no upstream change.

It is not recommended, and the reason is the owner's own sentence: *"we should still try to render a
correct image every frame"*. (e6) stops trying. It also does not reach 120 Hz — the readback is
still in the path — it still shows nothing on a repacked frame, and the moment the gesture ends the
event thread is frozen for the 231 ms of the frame that follows. It is a rate bought by abandoning
the other half of the requirement, and that is a decision for the owner rather than for a design
round. Written down here so that it is chosen or refused rather than forgotten.

**It is also in `stale.rs`, which session 550 is editing.** Nothing in this round touches that file.

## 4. What this side changes under the recommendation

Checked against `doc/ui-boundary.md`'s own test, against `CLAUDE.md`'s rule that `interpret` stays a
pure function of the bytes and the view state, and against the startup rules.

**`viewer-core`: nothing at all**, and this is the finding worth keeping. The boundary anticipated
an asynchronous renderer four hundred sessions ago: `Event::NeedsRender(RenderRequest)` carries an
`Arc<DisplayList>` and a `TargetSpec`, `Command::RenderReady { token, rendered }` closes the loop, a
stale token is dropped so "a worker that is slow costs a wasted render and never a wrong frame", and
rule 4 already forbids the core owning threads. `viewer-ffi` describes a render request as "an
opaque handle a caller may move to a thread of its own" — it is already documented as the thing this
round would do. **No `Command`, no `Event`, no `Query`**, so `doc/ui-boundary.md`'s test does not
fire, `PDFV_EVENT_KIND_COUNT` does not move, and no host fails to compile.

**`interpret` stays pure.** The render thread receives a display list and a target spec and never
sees a `Document`. Interpretation is already on its own thread at launch (`opened on its own
thread`); nothing about that changes, and the oracle's comparison rests on the same function it
rested on before.

**`render-quorra`**: `QuorraPresenter` splits into a renderer half (the `Device`, the
`ResourceCaches`, the `FrameSlot`) and a presenting half (the surface handle). Both `Send`.
`present` becomes a render into a texture; `capture_presented` and the whole readback path are
deleted rather than kept beside it. **The offscreen rasteriser is a different type and does not
move**, which is what keeps `doc/todo/37` rule 2 structural: the corpus gate and both oracle lanes
use `Target::Readback` and have no surface, so nothing that judges a picture can reach a blit.

**`viewer-ui`'s binary**: a render thread, a channel of requests and finished textures, and the
`Cadence` driving the presenting half. `Stale`'s base becomes a texture rather than an `Arc<[u8]>`,
which deletes the readback, the re-upload and the repack refusal in one change. The policy — the
five rules — is untouched and stays in `stale.rs`.

**The startup rules bind three things and forbid none of this:**

- the render thread is spawned when the **first render request arrives**, not in `resumed`. A
  spawn in `resumed` would put a scheduler decision in front of `graphics device`, which is a
  launch milestone;
- it never joins on the launch path, never calls `wait_until_warm`, and never blocks the event
  thread waiting for a first frame. The first present is still the first render's, arriving when it
  arrives — the launch timeline's *content* is unchanged;
- `detach_presenter` must compile nothing. If the blit pipeline is not warm at the first present it
  compiles inline, exactly as any first frame does. This is stated as a requirement in the ask
  (§6 of `doc/QUORRA_NONBLOCKING_RENDER.md`) rather than assumed.

## 5. The recommendation, with its number

**Ask quorra to split the surface off the device, and build (c′) behind the answer.**

The number: **1 present in 23 lands on the next refresh today (4.3%), and the median interval is
167.4 ms against a stated 8.333 ms refresh.** Under (c′) the only thing between the clock and the
screen is a swapchain acquire and one textured quad, and the page whose whole 58 009 commands
`execute` in 0.2 ms says what that costs. At 60 Hz the reprojections already fit (max 16.2 against
16.667); at 120 Hz they fit once the readback and the re-upload go, which is the same change.

**And the fallback is an honest one rather than a smaller version of the same thing.** If the answer
is no, the ceiling is the owner's measured median: presents spaced by the render, 167.4 ms, ≈ 6 Hz
on this document, with the reprojection improving *what* is shown at a view change and nothing about
*how often*. Neither rate is reachable and no work on this side changes it, because there is one
`&mut Device` between the clock and the screen. That would be recorded in `doc/todo/36` as a limit
with a name on it rather than as an open item.

The ask is `doc/QUORRA_NONBLOCKING_RENDER.md`. It carries §28.6's correction — `present_texture` on
`Device` would take `&mut self` and so be unreachable for exactly the 231 ms in which it is needed
— a smaller alternative (`Device::adapter()`), what it must not cost, what decides it, and the
`recording` question their own `doc/QUORRA_ENCODE_THREADS_ANSWER.md` §6 asked back.

## 6. What this round did not do, and says so

**It built nothing.** No line of code changed. Every gate was run anyway and every one is
unchanged, which is the only claim a design round can make about them; the session's own file
carries the figures.

**It measured nothing new.** Every number above is off `tmp/trace2.entwurf.txt`,
`tmp/trace3.entwurf.txt`, ADR 0368 and ADR 0383, read against quorra and wgpu at the revisions
`Cargo.lock` pins. The two places where a number is a subtraction rather than a measurement — the
steady-state reprojection, and the frame's cost with encode at zero — say so where they appear.

**And it could not have measured the thing that matters.** `Xvfb` states no refresh rate; 120 Hz
cannot be observed on this machine at all. That is ADR 0383's limit unchanged, and it is why the
ask names the owner's own display as the instrument.

## Clauses

**None**, for ADR 0378's, 0383's and 0384's reason unchanged: this is presentation and not a
reading. Nothing reprojected is a *rendering* of the page, so §10.7.4's scan-conversion rule does
not reach it, and the conformance ledger is unmoved.

## What did not move

Everything. `fmt`, `clippy --workspace --all-targets`, the workspace test run, the doctests and the
conformance checker were all run against a tree in which no `.rs` file differs from session 550's,
and all pass. The session's own file carries the figures.
