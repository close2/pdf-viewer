# 1968 — A lane the adapter refused is remembered for the device's life, and a refused pipeline by its frame

Session 1566, slot 3 of batch ninety. Status: **accepted, built**. Context: ADR 1821 (a refused page
is paid once), ADR 1929 (a pipeline the backend's compiler refuses is captured and kept), ADR 1962
(the compute lane keeps its refusals) and its section 4, which named this. Code:
`crates/viewer-ui/src/bin/quorra/renderer/lanes.rs` (`remembered`, `Kept`, `Refusals`),
`renderer/device.rs` (`draw`), `renderer/window.rs` (`Window::ask`, `Window::price`),
`renderer.rs` (`Known`, `Landed::not_asked`), `lanes.rs` (`upload_lane`). It cites no clause and
moves no ledger row.

## 1. What a refused pipeline cost

`quorra`'s window kept a refusal only where it was a fact about the frame — a budget, a limit, what
the display list carries — and asked the device again for every other kind. A pipeline refusal was
one of the others. Raster keeps that refusal for the device's life and answers every frame that
needs the pipeline in the same words without compiling again (ADRs 1929, 1962). But it answers
after the frame's scene is encoded, so the window paid the encode again on every frame and then
the processor's draw: 1 to 17 ms a page in ADR 1962's drive.

## 2. The decision

- **A refused compute pipeline is a fact about the coverage lane** (`Kept::Lane`). The compute
  lane's four pipelines are built or refused together on the first frame that takes the lane, so the
  refusal answers every later frame drawn under that coverage. The window keeps it beside the
  frames, for the life of the window's device, which is the render thread's and ends with the
  window. Every later frame of that coverage goes to the processor without the device being asked,
  whatever its pages and magnification.
- **Every other pipeline refusal is a fact about the frame** (`Kept::Frame`). That covers a render
  pipeline, a shader module and a generated §7.10.5 program, which raster refuses only for the
  frames that need them. These join the budget refusals in the existing memory, keyed by pages and
  magnification. A refused compute *module* is in this group too, because raster names it by its
  label and not by its lane. That costs one encode per frame rather than one per device, and it
  never sends to the processor a frame that did not need to go there.
- **The processor's picture goes back through a lane the adapter did not refuse** (`upload_lane`).
  The picture is one image plus the window's chrome. Under the refused lane, the chrome was refused
  with it. The drive in section 4 found this at HEAD: the sidebar was missing, and the terminal said
  `note: this page could not be drawn` at every frame of a page it had drawn. A refused `Compute`
  lane's picture now goes back under `Cpu`, and a refused `Cpu` lane's under `Gpu`, which sends
  nothing to the compute lane. This changes only the chrome of a frame whose lane was refused, and
  that chrome was not drawn before.
- **Said once, as before.** A frame of a refused lane carries the device's own words, so the title
  and the terminal's note do not change from frame to frame, and each is said once (ADRs 1466,
  1821). The trace's line for such a frame names the lane as the reason the device was not asked
  (`Landed::not_asked`).

## 3. The cost, written down

Under `Coverage::Cpu` on a real device, raster's compute assist (ADR 0090) sends to the compute lane
every tile the atlas does not hold. A refused compute pipeline heard there sends every later `Cpu`
frame to the processor, and that includes a frame whose marks would all have fitted the atlas,
which the device could still have drawn. The alternative is to keep the refusal per frame. That
pays the device's attempt once per page and magnification, which was 4 to 58 ms a frame in section
4's drive. The processor is the correctness oracle and draws the same page, so nothing drawn
changes, only the lane that draws it. Frames under another coverage are still asked of the device.

## 4. Held by

- `lanes::tests::only_a_refusal_about_the_frame_or_the_lane_is_remembered` holds the sorting: a
  compute pipeline goes to the lane, and a render pipeline and a compute module go to the frame.
- `lanes::tests::a_refused_lane_answers_every_frame_of_its_coverage_for_the_device_s_life` holds the
  memory: another page at another magnification is not asked, thirty-three frame refusals do not age
  the lane out, and a frame of another coverage is asked. With the lane's keeping planted out, it
  fails at its first assertion (trap 13).
- `gesture::an_interrupt_raised_before_the_processor_starts_abandons_the_frame` carries a lane
  refusal, and the device is not asked.
- **Driven**, under an Xvfb of the round's own with `--coverage compute`, on `tracemonkey.pdf` and
  `PDF20_AN001-BPC.pdf` (three page turns, two zooms in, one out, one turn back). ADR 1962's plant
  was used: an environment variable breaking the count pipeline, in two exports only. HEAD with the
  plant asked the device for every frame: 8 frames refused in 6.7 to 57.6 ms, 157.5 ms in all, and 9
  frames in 4.2 to 7.6 ms, 52.1 ms in all. The second document opens with its outline in the
  sidebar, and its run also said `this page could not be drawn` at all 9 frames. This change with
  the plant asked once, 8.5 and 7.5 ms. Every later frame was `not asked: this adapter refused the
  pipelines of its coverage lane`. The chrome was drawn, and nothing said the page could
  not be drawn. The note and the one-line title came once in each run. Without the plant, the same
  build drew every frame on the device. Before the upload change, the planted capture was identical
  to HEAD's planted capture (0 pixels differ in each document).
