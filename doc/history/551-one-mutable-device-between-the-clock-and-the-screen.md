# 551 — One `&mut Device` between the clock and the screen

2026-08-16. A **design** round on the project owner's question, against their own traces from an AMD
Radeon 890M under RADV on Wayland. **It built nothing**: no `.rs` file differs from session 550's.
It ends in ADR 0386, in `doc/QUORRA_NONBLOCKING_RENDER.md`, and in one new section of
`doc/todo/36`.

## The question

> *will we be able to achieve either correct or reprojected frames (for every frame)?*

60 Hz as the floor, 120 Hz as the target.

## The answer

**Yes at both rates, and only with a change on quorra's side.** Everything this tree can do alone
has been done over sessions 547 to 549; what is left is one sentence of ownership —
`quorra_gpu::Device::render` takes `&mut self` and the same object owns the surface, so the
reprojection that exists for exactly the 231 ms a frame costs cannot be issued during them.

## The arithmetic, per rate

Off `tmp/trace3.entwurf.txt`: `tmp/Entwurf.pdf`, 58 009 commands, a 1280×1600 window, a display
that states 120 Hz itself.

- **60 Hz** needs a non-blocking render and **nothing else about cost**. All seven of the owner's
  reprojections fit 16.667 ms; the worst is 16.2, which is a fit by 2.8%.
- **120 Hz** needs the non-blocking render **and** `doc/QUORRA_FEEDBACK.md` §28.6's no-readback
  path: six of those seven miss 8.333 ms, and the readback (2.7–6.6 ms) is the whole of the
  difference. That asymmetry had not been stated in one place before.
- **A third requirement nobody had written down.** A reprojection needs a base, and the owner's run
  printed the refusal twice against its seven reprojections — the atlas repacked and the retained
  encode died with it. A page rendered into a texture the host owns is unaffected by a repack, so
  the same change closes this too.
- **The ceiling, so the ask is not oversold.** During the 4.366 s the owner's view was moving,
  120 Hz is 524 refreshes and **15 could carry a rendering — 2.9%**. The reprojection is a floor and
  it is most of what would be seen.

## The number that reframed the options

`execute` — the device's own timestamps over the drawing passes — is **6.7 ms of 4454.9** across the
owner's whole 24-frame run. **0.15%.** The graphics device is idle for essentially all of a frame of
this page; a frame is a processor running on the calling thread. That declines "submit, then poll"
on a measurement rather than a preference, and it is why the seam has to be at the surface.

## The options, priced

| | buys | verdict |
|---|---|---|
| a non-blocking `render` upstream (submit, poll) | 0.15% of the frame | declined on the measurement |
| the encode on quorra's own pool | a cheaper frame, never an asynchronous one — 26 ms at the median even at zero encode | keep asking; not this question |
| a render thread here with a **second device** | nothing that pays for itself: **8 192 000 bytes a frame** across, plus a second bring-up and duplicate residency | priced and declined |
| **a render thread here with the same device, split at the surface** | **the only option that reaches either rate** | **recommended** |
| chunked/resumable encode | nothing without the split, 28 re-entries with it | dominated |
| tier 3 with the host owning the surface | would work — if the format could be learned | blocked at wgpu's adapter chain |
| defer the real frame until the view rests | 60 Hz through a gesture, today, no ask — and no correct frame during one | available; it trades the owner's own sentence, so it is theirs |

## What was verified rather than quoted

Against quorra at `a4380e2c` and wgpu 30.0.0, which is what `Cargo.lock` pins:

- `Device::render(&mut self, …)` and `render_retained(&mut self, …)`, with `surface` a private field
  beside the caches; the whole public surface of `Device` carries **no adapter accessor and no
  constructor that adopts an existing `wgpu::Device`**;
- `Surface::get_capabilities` and `get_default_config` both take `&wgpu::Adapter`; `wgpu::Device`
  gives only `adapter_info()`. So a host cannot configure a surface of its own, and a second quorra
  device is a second wgpu device whose textures are not the first one's;
- quorra's `encode/parallel.rs` threads `flatten` and `fill_mask` and nothing else — the walk and
  the commit are serial *by design*, which bounds what more threads can be worth.

## What this side would change

**`viewer-core`: nothing.** The boundary anticipated an asynchronous renderer four hundred sessions
ago — `NeedsRender` carries an `Arc<DisplayList>` and a `TargetSpec`, `RenderReady` closes the loop,
a stale token is dropped, and `viewer-ffi` already documents a render request as "an opaque handle a
caller may move to a thread of its own". No `Command`, `Event` or `Query` moves;
`PDFV_EVENT_KIND_COUNT` does not change; `interpret` stays a pure function of the bytes and the view
state. The work is `render-quorra`'s window-owning presenter split in two and a render thread in the
binary — spawned at the first render request, never joined on the launch path, never waiting for
warmth.

## The ask

`doc/QUORRA_NONBLOCKING_RENDER.md`, in the house voice. It carries the correction to our own §28.6
— **`present_texture` on `Device` would take `&mut self` and so be unreachable for exactly the
231 ms in which it is needed** — a smaller alternative (`Device::adapter()`), what it must not cost,
the three things that decide it with the third being ours, what happens if the answer is no, and the
`recording` question quorra's own `doc/QUORRA_ENCODE_THREADS_ANSWER.md` §6 asked back.

## Gates

**No code changed, so nothing could move; every gate was run anyway and that is the only claim a
design round can make about them.** `fmt` clean; `clippy --workspace --all-targets` silent of Rust
lints; **2036 tests run, 2036 passed, 15 skipped**; doctests pass; conformance passes (5 tests).

One thing the run taught about the harness rather than the tree: five `pdf-font` tests failed in a
fresh worktree until `doc/*.pdf` were symlinked. Their corpus is `doc/*.pdf` — the ISO documents
themselves — and `doc/md` is not the same setup step. Recorded here because two rounds of the
five-hundred-and-fifties will hit it.
