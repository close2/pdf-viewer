# 790 — The device behind the confined window, and the identity that had to survive the pipe

The batch's `doc/todo/15` round. Of the boundary's three owed pieces — the warn-before-abort
input for the three established windows, the quorra surface behind the confined window, and
nothing (ADR 0718 §2's retry design turned out to be priced-and-refused, a record rather than
work) — this round took the largest: **`pdf-viewer-confined` now presents through
`render-quorra`** (ADR 0725), so a page whose content arrived through the confined pipe is drawn
by the graphics device, which is what ADR 0607 argued the marks cross the pipe *for*. The
confinement moved nowhere: the worker still ships display lists and rasters over the pipe, and
the device is the host's.

## The finding on the way in

The `Arc` identity two documents promised — `Payload::List`'s "[s]hared because a host keeps it"
and the screen's same-drawing-moved reuse — **did not survive the pipe**: every `Query::Frame`
decoded a fresh `Arc<DisplayList>`, so the reuse arms fired only in unit tests that shared the
`Arc` by hand, and the live window re-rasterised every marks page on every scroll. Fixed at the
transport: `Confined` now holds, per page, the encoded bytes and the `Arc` they decoded to, and
byte-identical re-crossings hand back the same `Arc` with the decode skipped
(`protocol::HeldLists`; bounded by the frame on hand; host side of the pipe). Without this the
device's retained scenes — keyed by those very addresses — would have rebuilt every frame.

## What moved

- `crates/viewer-confined/src/protocol.rs`, `lib.rs` — `HeldLists`, `decode_answer_reusing`,
  `Confined::held`; two new tests, each calibrated (defects injected: reuse bypassed, eviction a
  no-op). No wire-format change; `MAGIC` unmoved; the fuzz targets still compile (§2's check line).
- `crates/viewer-ui/src/bin/pdf-viewer-confined/device.rs` (new) — the flagship's render-thread
  arrangement without its reprojection machinery: presenter on the event thread, renderer moved
  by the first job, one job in flight with the newest ask replacing a waiting one, textures
  travelling as a pooled pair, `Ungrounded::ground` configuring the surface before anything can
  submit. A device refusal is **not** composed on the render thread (deliberate departure from
  the flagship): it comes home as `Landed::refused` and the marks go to the interruptible
  drawing thread — ADR 0650's interrupt is why.
- `crates/viewer-ui/src/bin/pdf-viewer-confined/screen.rs` — the device screen: marks kept as
  `Content::Marks` for the device, worker rasters wrapped once as one-`Image` lists whose `Arc`
  survives byte-identical re-crossings, `fall_back` for refused frames, `device_pages` placing
  each page with `blit`'s own rounding. Three new tests, each calibrated.
- `crates/viewer-ui/src/bin/pdf-viewer-confined.rs` — `--cpu` (the flagship's flag and meaning:
  no instance, no driver), instance on a thread from construction, bring-up with fallback to the
  software path out loud, ask/collect/present flow, launch settle on the first device frame.
- `doc/adr/0725-the-device-behind-the-confined-window.md`, `doc/todo/15`, `doc/state-of-play.md`.

## Proof (Xvfb :179/:183, llvmpipe, release binaries; illustration, not a gate)

Marks arm: `PDF20_AN001-BPC.pdf` device up in ~44–50 ms, first frame presented ~0.10–0.11 s;
captures show the cover and page 3 at 2× — sharp, placed on the surround. Raster arm:
`personwithdog.pdf` wrapped and placed; scrolls at 2.9–7.1 ms a device frame (the wrapper
identity holding). §7.6.4.1: the card drawn by the device's chrome lane; the right password
opens through the marks arm; no password legible in the trace. Abort: the 1567-byte
amplification fixture (level 4, marks arm) — the device digests at ~2.8 s a frame under llvmpipe
what held the CPU rasteriser 27.6 s, off the event thread; Escape kills the worker, no zombie,
exit on `q` in 260 ms. Control: `--cpu` renders the same cover through the drawing thread,
unchanged. **Owed, not guessed**: the real adapter's bring-up and present cadence need the
owner's session; recorded in `doc/todo/15`, and the device lane's coverage choice stays quorra's
default until a measurement asks.

## Gates (fifth round: the full sequence, run here after the final edit)

`cargo fmt --check` clean; `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`
clean; nextest **2736 passed, 18 skipped**; doctests green; fuzz `check --bins` green; sandbox
and `pdfref-hayro` built; corpus gate ok; oracle ok (3 passed, 146 s, quiet machine);
text-extraction ok (4 passed; word boxes 98.26% in bounds); selection census ok; accessibility
census ok; dates, xmp, jpeg2000 ok; quorra corpus ok; fixed documents ok; conformance **200
passed**. §5's eight artifacts rebuilt in release and installed into this worktree's `target/`
from the directory `cargo metadata` names.

## Sweeps (§4, before on pristine f81e038f, after at the end; both in one sitting)

- pointers 8925→8930 path (live 5081→5086; absent 98 unchanged) — the round's own new files and
  citations, re-run after this file was written.
- quotations 6648→6652 in 1061→1063 documents — ADR 0725 and this file, both quoting this
  tree's own sentences (the "unrelated" bucket); verbatim/diverging unchanged.
- blockers 41→42 (naming-no-clause 11→12) — the one new hit is
  `pdf-viewer-confined/device.rs:1`'s module sentence about one job in flight: a design sentence
  in the sweep's stated noise shape, read and accounted.
- overstated unchanged (62/8); overtaken 613→614 decision records (ADR 0725), 47 unchanged;
  parts unchanged (586/39/547); tables unchanged.

## Trap-13 calibrations

Six new tests, each watched failing against its own injected defect before being believed; the
table is in ADR 0725.

## Contradictions with the briefing, and small observations left for a later round

- The briefing listed "ADR 0718 §2's refused retry design" as a remaining item; the tree says it
  is a decision already made (priced and refused in that ADR) — nothing was owed.
- Observed, not changed: the screen's `Content::Refused` is kept for *any* later list payload of
  the page, even one with a new target — a refusal at one magnification outlives a zoom. Both
  the processor path (since ADR 0713) and the device fallback inherit it; worth a look from a
  round in that area.
- The `frame_landed` trace prints two device frames per input — the `Damage` event's ask and the
  frame pull's ask, the second superseding the first, both cheap under scene reuse. Left as is.

Remainder of `doc/todo/15`: the warn-before-abort input for the three established windows
through `viewer_host::keys`; the breach-as-refusal item; moving the established windows onto the
boundary; and the real-adapter measurement above.
