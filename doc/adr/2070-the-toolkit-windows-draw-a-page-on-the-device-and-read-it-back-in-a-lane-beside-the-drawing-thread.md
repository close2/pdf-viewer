# 2070 — The toolkit windows draw a page on the device and read it back, in a lane beside the drawing thread

Session 1617, slot 3 of batch ninety-seven, a "pixels" round on ADR 2049's G1 and Q1. Status:
**accepted** and **built**. Context: the owner's `doc/questions/A287` ("If possible gtk, qt, and
dotnet should provide a way to still render on the GPU"); ADRs 0244, 0246, 0668, 1831, 1855, 2049,
2052, 2053, 2055. Code: `viewer_device::renderer::ReadBack` (`renderer/read_back.rs`);
`viewer_host::drawing::{PageLane, Handed}`, `Drawing::finishes_ahead`, `Ahead::holds`,
`WindowState::{lane, drawing_interval}` and `NativeWindow::take_the_lane`; `device_lane` and
`Host::draw_on_the_device` in `viewer-gtk` and `viewer-qt`, and `--cpu` on both. Cites no clause.

## 1. What the premises said, and what held

- **"The windows render through `viewer-device` and read the frame back": held, as a page and not
  as a frame.** A toolkit window is handed a page at a time (`Event::NeedsRender`) and owes the core
  a `Rendered::Raster` for each; it composes the arrangement itself, from its own widgets. So what it
  reads back is a page, drawn by `QuorraRasterizer` into a `Readback` target, which is the same
  `Raster` the processor hands over. The presenter, the window frame and the five stand-in rules of
  `viewer_device::stale` are `quorra`'s and stay so; the toolkits' stand-ins stay
  `viewer_host::stand_in`'s.
- **"A host supplies `Wake`, `FrameLines` and `FrameClock`": one of three held.** `FrameLines` and
  `FrameClock` serve the stand-in policy of a window that presents layers, which a toolkit window does
  not run. `Wake` held in GTK: `glib::MainContext::invoke` from the device's thread, which finds the
  window through a thread-local on GTK's thread. **In Qt it did not hold**: Rust never calls a Qt
  object, and a call from the device's thread into `QApplication::exec` is the second hand-written
  `unsafe` token `crates/viewer-qt/src/bridge.rs` exists not to have (ADRs 0470, 0519, 0526). The Qt
  window takes the device's pages on the drawing poll it already has, whose interval is the device's
  while a page is out (`WindowState::drawing_interval`). The brief's hypothesis that "the wake is the
  only piece each toolkit shapes differently" held in that sense: it is the only one, and one of the
  two shapes is no wake.
- **"`viewer-device` additive only": held for the crate; a seam was needed in `viewer-host`.** The
  ask and the answer for a page are `NativeWindow`'s provided methods (`needs_render`,
  `take_the_drawn`), shared by both windows; written in each host they would be the second and third
  copies of the one function `doc/todo/30` says hosts stop agreeing at. So `viewer-host` gained a
  trait, `PageLane`, and a field for one; `viewer-device` implements it. `viewer-host` cannot name
  the device itself: the confined worker's graph runs through it (ADRs 2052, 2053).

## 2. The decision

1. **One lane beside the drawing thread, offered every page first.** `needs_render` offers the
   request to `WindowState::lane`; the lane takes it, or hands it back and the drawing thread draws it
   as before. `take_the_drawn` takes the lane's finished pages first and answers them by the same loop
   as the drawing thread's, so the retained pictures, the oversized windows and the core's answer
   cannot tell the lanes apart. A window with no lane — `--cpu`, or a host that never sets one — is
   byte for byte the arrangement before this record.
2. **The lane is chosen by `quorra`'s own records.** `ReadBack` holds a `Costs` and a `Refusals`
   (`renderer::lanes`) keyed by the page as `quorra` keys its frames: inside `WARN` the device draws,
   past it the smaller prediction, and a refusal at a magnification or of a coverage lane is not asked
   again. The coverage lane is `lane::coverage_for`'s launch rule, per page. ADR 1831's sentence goes in
   the title and on the terminal when it changes, in the same words (`Prediction::said`).
3. **What the device does not draw, the drawing thread draws**: a page refused, a page predicted
   faster on the processor, every page with no device, a page zoomed past its window (ADR 1930's
   windows are asked of the drawing thread directly and are not offered), and a page the drawing
   thread finishes from its drawing ahead (`Drawing::finishes_ahead`, ADR 1855), which is all but
   drawn already. A device page cannot be taken back (ADR 0725), which is why the lane past `WARN`
   is the processor's: that draw can be shown as it is built and stopped.
4. **The device comes up beside the document's read** (`ReadBack::bring_up`, called where the
   binary starts the open), off the window's thread; nothing waits for its pipelines, and no page is
   drawn on the processor to cover the bring-up. A machine that admits no device is told on the
   frames trace, and the processor draws every page saying nothing new (ADR 2049 section 4).
5. **The toolkits' default lane is the device**, ADR 2049 section 4's recommendation, decided by
   ADR 2071 on the launch figures.

## 3. What it costs

A second device in the process, beside GSK's or Qt's own — `raster-gpu`'s first block is 128 MiB
under wgpu's default hints — and `wgpu`, `naga` and `ash` in both toolkit binaries. One copy per
page from the device's readback into the `Raster`, then the toolkit's own copy into its texture, as
before. `viewer-host` holds a trait object it cannot see into, which is the price of keeping the
device out of the confined worker's graph. ADR 2053's rule now admits four manifests, and the two
toolkit crates are depended on by nothing (`cargo tree -i`), so no judge links the stand-ins.
