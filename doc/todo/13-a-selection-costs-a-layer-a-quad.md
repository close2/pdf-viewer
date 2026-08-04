# A refused frame leaves the window blocking one second a present

Status: **the selection's cost is fixed (ADR 0176). The refused frame's recovery is not.**
Priority: 13 — what is left is reachable by any frame the device refuses
Corpus: none. This is ours, in the host.
Clauses: none.
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`present`)

## What was reported, and what half of it was

> If I open `issue14821.pdf` and select text randomly (by dragging the mouse over the page), the
> application becomes unresponsive. I sometimes were able to "reset" it by changing the window
> size, but not always (or I didn't wait long enough).

**The selection's half is closed.** `highlight_list` drew one `Multiply` fill per quad, the
compositor gives every non-`Over` blend its own layer and prices them all before allocating any,
so a quad cost 6.4 MB of frame budget at 800 × 1000 and 63 of them — one short paragraph —
spent the 256 MiB. One fill of one path with one subpath per quad draws the same pixels under two
layers instead of sixty-four. Measured in the window afterwards: **268 quads presented in
16.5 ms**, against 37 quads in 105.6 ms and a refusal at 63 before. ADR 0176.

`--trace` prints `SELECTION quads N` now, which is the number the whole diagnosis turned on.

## What is left, and it is the defect the report was actually about

After a refused frame, **every subsequent present blocks for exactly one second** and returns
`SurfaceProblem::Timeout`, for ever. The process sits at 4% CPU, so it is blocked rather than
spinning. A resize recovers it; nothing else does.

**Why**, from reading `quorra-gpu` at `3f45555` — offered as a reading, not as something this
tree has instrumented:

- `Device::render` calls `bind_target`, which for `Target::Surface` **acquires the swapchain
  texture**, and only *then* prices the compositor's internal textures and returns
  `FrameBudgetExceeded`. A refused frame therefore drops a `wgpu::SurfaceTexture` that was never
  presented, and whose acquire semaphore no submission ever waited on.
- `Surface::acquire` sets `needs_reconfigure` for `Suboptimal` and `Outdated` and **not for
  `Timeout`**. So nothing reconfigures the surface again — and a host resize changes the
  configured size, which is exactly why a resize is the one thing that recovers the window.

Both are written up in `doc/QUORRA_FEEDBACK.md` §7 as the offer across the boundary.

## What this side owes, whatever the library does

**A host that stays answering when the device says no.** The fix above removed the one refusal a
person meets by using the program normally; it did not make the *next* refusal survivable, and one
is still reachable — a page whose scene the device cannot draw in one pass is what `render-gpu`'s
banding exists for (ADR 0127) and quorra has no equivalent of.

What has to be settled before it can be taken:

1. **How a host reconfigures a surface it does not own.** `QuorraPresenter` exposes `present` and
   nothing else; the size is taken from the frame. Presenting one frame at a size off by a pixel
   would force the reconfigure and is a hack that would have to be written down as one. Asking
   quorra for the entry point is the better half of the offer.
2. **Whether the CPU fallback should drop the overlays.** `present` falls back to a CPU raster and
   re-presents it with *the same* overlay lists, so a frame refused because of an overlay is
   refused identically on the fallback path. Dropping the chrome for one frame would at least put
   the page on screen. Not obviously right — a selection that disappears when a page is heavy is
   its own defect — which is why it is a question and not a plan.

## Two measurements this file should keep

- **The quads do not need coalescing.** One rectangle per selected line rather than one per run
  would shrink the path, and the layer count is what the budget turned on, so it buys nothing that
  has been measured. It would also change what `Answer::Selected` means, which `headless.rs` pins.
- **The per-quad blend preserved nothing.** `Query::Selection` answers one quad per run and runs
  tile: three lines of `tracemonkey.pdf` give 19 quads with **two overlapping pairs out of 171**,
  by 0.28 and 0.17 of a device pixel. The comment claiming otherwise was an assertion nobody had
  checked, and it cost this project its only user-visible defect.
