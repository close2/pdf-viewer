# A frame every refresh — 60 Hz as the floor, 120 Hz as the target

Status: **the cadence is built; the frame is now measured, and what it costs is the open half.**
The presenter is a clock on a thread of its own, a view that keeps moving is answered every tick,
a missed frame is stood in for by a reprojection and a late one re-bases (ADRs 0383, 0384, 0385,
0391). What was never measured until ADR 1260 is the thing the whole design is a floor under:
**what one frame costs, stage by stage, against the 8.333 ms a 120 Hz refresh allows.** It is
measured now, and the answer decides what the next rounds take.
Priority: 36 — **and `36` is this file alone since ADR 0983**, which moved the other item that
carried the number to `doc/todo/63-a-retrieval-api.md`; a citation of `doc/todo/36` about a
retrieval CLI or a `Query` for a page's text means that file rather than this one.

Instruments, and neither carries its numbers in a document:

- **`tools/state.sh frame`** — what a frame costs, stage by stage, for three page classes at two
  magnifications: the page *turned to*, the same frame asked for again, and the page placed at
  twice the magnification. `crates/render-raster/examples/frame_budget.rs` is the example under
  it and its module comment says what each row contains. ADR 1260.
- **`quorra --trace`'s frame lines and its summary** — the cadence in a real window: the interval
  distribution, and what share of the presents were the page rather than a picture of it moved.
  It needs a display server; `doc/environment.md`'s `Xvfb` recipe is how it is run here.

Clauses: none — presentation. §10.7.4 does not reach it: nothing reprojected is a rendering.
Code: `crates/viewer-ui/src/bin/quorra/{cadence,stale,surface,window}.rs` for the clock,
`crates/render-raster/src/scene.rs` and `crates/pdf-model/src/content/` for what a frame is made
of.

## What the measurement says, in shapes rather than numbers

Run the instrument for the figures. What it establishes, and what a later round should re-derive
rather than inherit:

- **A repaint of what is already on the screen fits inside one refresh on every page class
  measured**, with most of the budget to spare. A window that has drawn its page can answer a
  selection, a caret or a chrome change at 120 Hz today.
- **A page turn onto a page whose outlines the device has not seen fits on none of them**,
  including a page of ordinary text. That is the expensive end of the gesture; `launch_path`'s
  own page-turn figure — arrow keys inside one document whose page one is already drawn — is the
  cheap end, and a session moves between the two.
- **The largest stage is a different one per page class** — raster's encode for dense text, this
  tree's own scene walk for vector artwork, interpretation and the transfer for a photograph — so
  no single witness can rank this work, and the population is part of the claim.
- **The graphics device's own passes are a few per cent of every row.** A frame that misses the
  refresh is a host thread every time, which is what makes each remaining item a host item.

## The two halves

**Settled: what happens when a frame is late.** The clock, the reprojection, the re-basing, the
base that outlives its frame, and the refusals that print which of two kinds they are. The owner
has chosen *correct frames wherever possible, reprojection where not*, and declined the deferral
that would buy 60 Hz by not trying — **do not re-open that as an optimisation**. ADR 0386 §3.3 is
the trade they refused; a later round that finds a gesture smoother when the real frame waits is
rediscovering it, and owes this paragraph an argument rather than a measurement.

**Open: where the pixels come from in 8.3 ms.** This is the half the budget now describes, and it
is four costs with two owners:

| the cost | whose | where it stands |
|---|---|---|
| raster's encode of a page seen for the first time, on the lane a page turn takes | quorra's | asked, with the measurement: `doc/QUORRA_FEEDBACK.md` §52 ask 1 |
| an image restaged for every placement it is drawn at | quorra's | asked, with the byte counts: `doc/QUORRA_FEEDBACK.md` §52 ask 2 |
| a mesh shading rasterised into device pixels on every view change | this tree's | the **paint** is divided across the pool (ADR 1259); `PatchMesh::tessellate` is still serial and is what remains |
| a photograph decoded on the way into a page turn | this tree's | **taken** (ADR 1271): the decoder is asked for the raster this tree used to widen its components into, and the walk that looks for a `DNL` marker reads the codestream a word at a time rather than a byte. What is left of that stage is `zune-jpeg`'s own Huffman and IDCT, which is most of it and is nobody's to divide — `doc/stack.md`'s crate is single-threaded by construction |

## Two standing facts about measuring this, which no command prints

- **120 Hz cannot be observed on this machine at all.** `Xvfb` states no refresh rate — `xrandr`
  reports `0.00` and `--newmode` does not take — so a window run here takes the 60 Hz floor
  whatever the code does, and a cadence claim about the target has to come from the owner's own
  display. What is establishable here instead is the pair ADR 0383 rests on: that the presenter
  sustains the intervals when the frames fit, and which kinds of frame fit.
- **A window dragged to another display keeps the first display's cadence**, and the honest fix is
  upstream rather than here. `Cadence::ask` stops at the first answer from the window's own surface
  deliberately (ADR 0384 §5), because polling the monitor every frame is a per-frame cost for a
  question that changes when somebody drags a window; winit's Wayland backend *receives*
  `surface_enter` and its handler body is empty, so nothing tells this tree the output changed.

## What the next round takes

The two that are this tree's, in this order:

1. **`PatchMesh::tessellate`.** Each patch is independent and produces its triangles in order, so
   ADR 1259's argument carries — but it needs its own measurement, because the paint was the half
   that was measured and the tessellation is what is left of that stage.
2. **A page's images decoded in parallel.** Every image `XObject` a page names is decoded
   independently and the pool is already there, so a page of many photographs is divisible where
   one photograph is not — and nothing incorrect is ever presented, which is what separates this
   from the deferral ADR 1272 prices and `doc/questions/Q121` puts to the owner. It is a change to
   how the interpreter walks a content stream rather than to how a codestream is decoded, and it
   needs a witness page with several images to be measured on.

Both are wall-clock changes on a shared machine, so both owe `doc/habits/measuring.md`'s method:
arms alternated in one sitting, the minimum of several fresh processes, the load average printed
beside every figure.

## Why the owner's framing is right, and worth keeping

*"We should still try to render a correct image every frame"* — the reprojection is a floor under
the experience, never a substitute for the frame being fast. Every round that makes a frame cheaper
reduces how often this item is visible at all, and the ceiling ADR 0386 measured on the owner's own
trace — the share of a moving view that can ever carry a rendering — moves with it. That is the
whole road, and the budget above is the map of it.
