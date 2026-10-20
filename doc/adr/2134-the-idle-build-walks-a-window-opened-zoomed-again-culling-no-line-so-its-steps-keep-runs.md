# 2134 — The idle build walks a window opened zoomed again, culling no line, so its steps keep runs

Session 1649, slot 3 of batch one hundred and one, `doc/todo/75` item 3's first remainder, on the project
owner's ask of 2026-10-19 that a zoom "send only a matrix" and the switch to code improvements of
2026-10-14. Status: **accepted** and **built**. Context: ADRs 0087, 1289, 1898, 2030, 2052, 2056; round
1610's "Unfinished". Code: `raster-gpu`'s `kept_lines::Before::Whole`, `Unclipped::whole_under`,
`encode::encode_whole`, `RetainedScene::{wants_whole_walk, prepare_whole}`, `Device::build_whole`;
`render-raster`'s `FrameSlot::build_whole`, `QuorraWindowRenderer::{build_whole, page_wants_whole_walk}`,
`QuorraRasterizer::build_whole`; `viewer-device`'s `renderer::device::{build_whole, next_step_lane}` and
the render thread's last idle step. Tests: `raster-gpu`'s
`the_idle_build_of_a_window_opened_zoomed_keeps_the_lines_for_the_step` and
`the_idle_build_is_declined_once_where_it_would_keep_nothing`; `render-raster`'s `tests/idle_build.rs`.
Instrument: `zoom_frame`'s `ZOOM_FRAME_IDLE`. Cites no clause and moves no ledger row: the standard says
nothing about when a viewer walks what it draws from.

## 1. The premise, and the hypothesis

**The premise held**: a walk with no encode of the scene before it culls the lines its view admits
nothing of (ADR 2056 §2.1), so a window opened at a magnification keeps no runs and every step from it
walks the page's 4.4 million lines again. **The hypothesis — forty lines in the render thread and one
message — did not hold in its size.** No entry of raster's walked a scene culling nothing at a view that
does not show it; the thread's idle step needs one, and the encode it makes has to reach the device and
take the held encode's place, which is raster's `RetainedScene` and `render-raster`'s `FrameSlot` and not
the thread's. The thread's part is thirty lines; no message is sent, because nothing the window decides
reads whether the device keeps the lines. **And the brief's figure was not its instrument**:
`viewer-host`'s `window_against_whole` draws through the processor's drawing thread and never reaches the
device; the device's zoom steps are `render-raster`'s `zoom_frame`, which ADR 2056 measured with.

## 2. Decision

1. **The idle build is a walk at the view the person left the page at that culls no line**
   (`Before::Whole`): a line its view admits nothing of writes its instance with an empty box and grows
   no plan, which is what the vertex stage draws of it there. Its frame is therefore the first frame's to
   the pixel, and its stream is the page's, kept as runs by ADR 2056's own construction.
2. **It is drawn, into a texture nobody presents**, rather than encoded alone: the frame is what
   charges its streams against the residency budget and makes them resident (ADR 2030), uploads the atlas
   rows and compute records the encode names, and settles the atlas — every one of which a frame already
   does and an encode left undrawn would have to do again by hand.
3. **It takes the held encode's place only where it is worth holding**: not where the held encode is
   this view's and keeps its runs, nor where the walk kept none (a line inside a group or a soft mask),
   nor where it wrote records — a step from such an encode takes the record road, which walks the lines
   again (section 5) — nor where its streams would take the device past its residency budget beside what
   it keeps; the held encode then stays, and the scene is not asked again until it changes.
4. **Last in the idle turn, under the lane the step will take.** The sharp pass draws the view at twice
   the window and the retained layer draws the page small, each through the page's own scene, and each
   puts its encode in the page's place; built before them, the build would be gone by the next step. The
   step from a still view is a moved view, which `lane_for` gives the compute lane on a device worth the
   name (the session's lane where it is fixed, the frame's on a software adapter); an encode is kept for
   the next view only on the same lane.
5. **It waits out the frame it repeats first**, the sharp pass's rent-or-buy bound (ADR 1289): it cannot
   be taken back once begun, so a view change arriving while the view has been still for less than the
   build costs is drawn instead. It is asked only by a frame the device drew and whose encode the next view
   would walk again; nothing before the first frame asks it (principle 2), and the first frame is HEAD's.

## 3. Measured

One `--clock` hold (load 3.2–5.6, the Radeon 890M through RADV, headless, `Compute` coverage), HEAD's
export against the change's, interleaved, two passes at 4 and at 24 encode threads, minimum of three
rounds a process; `zoom_frame` with `ZOOM_FRAME_WINDOW=900x1100`, scale 0.3775, the sequence 2.4, 3.0,
3.6 — a window opened at 2.4 times the fit — and `ZOOM_FRAME_IDLE` in the change's arm. Milliseconds,
the frame's total and in brackets its `encode`:

| the witness, a window opened at 2.4 | HEAD | this change | bytes sent, HEAD → change |
|---|---|---|---|
| the first frame | 532.2–554.9 | 531.4–574.4 | 132.7 MB → 132.7 MB |
| the idle build, while nobody waits | — | 259.7–276.4 | 264 MB of lines, resident |
| then 3.0 | 154.6–194.0 (105.4–124.4) | **121.3–131.4 (58.5–59.8)** | 84.2 MB → 18.8 MB, no line byte |
| then 3.6 | 143.8–174.8 (101.7–107.2) | **118.3–124.7 (59.4–63.5)** | 64.6 MB → 21.2 MB, no line byte |
| from the fit, then 2.4, then 3.0 (24 threads) | 134.9, 124.1 | 138.5, 120.5 | the same |

**A window opened at a magnification now steps as a window opened at the fit does**: 19–73 ms a step
faster, its encode halved, every one of its 4 398 932 lines kept. The hypothesis's absolute (about 90
ms) is the fit's step as ADR 2056's hold measured it; this hold put the fit's step at 120–139, and the
opened window's steps beside it. **The first frame is HEAD's**: the same digest (`e0ff5b9a63ba9363`)
in every round of both arms, its time within the machine's spread, its bytes the same; the idle build
draws that digest too, and every step's digest is HEAD's at its view. Four and 24 encode threads read
alike, since the walk the runs leave is not the line makers'. The idle build's time includes
`zoom_frame`'s read-back, which a window's texture target does not pay; what it costs a window is a
moment of the render thread after the view has been still that long, and the page's line stream held
on the device (264 MB where the first frame's was 114), inside the residency budget or not built.

## 4. The proof that nothing else moved

The four device arms of the change's export against HEAD's (`/home/AI/arms-1647/`), by page: gpu and
compute at 1× (968 pages each) and 4× (965), 0 differing and 0 one-sided — a corpus page is one frame,
and no first frame asks for the idle build; the comparison names 965 of 965 when handed the 4× arm as
the 1×. `pdf-model`'s `raster_golden` 2 passed (its graph holds none of the three crates, `cargo tree
-i`). The turn gate's 33 figures inside their bands in both trees, in the same hold; the launch gate
(`PDFVIEWER_LAUNCH_CLOCKS=1`) 53 figures banded, 0 outside, 9 not judged — seven for the adapter's
carve-out, one for the disk, one for a processor wait — on the change. Planted back, the tests fail: the
idle build culling as the walk does (raster's test), the render thread never asking for it (the
thread's).

## 5. What is left, and the record replay

- **The record replay keeps no runs, and is not owed them yet.** A page every one of whose commands a
  record replays (ADR 0087) — lines, rectangles and solid fills under rectangles alone, on the compute lane
  — steps by its records, re-dispatching each line through the walk; keeping runs there would be ADR 2056's
  kept walk written a second time over the record list, and which of the two roads is the faster for a page
  of both is a question with no page measured on it: the witness leaves the record road at its 26th
  command, and no row of `doc/checks/turn-path.toml` is a replayable page of lines. The idle build declines
  such a page rather than build for nothing. Owed where a census finds a replayable page whose steps walk
  lines.
- **The vertex stage draws every kept line in a kept frame** (0–4 ms of `execute` on the witness, ADR
  2056 §5): drawing a run through the indices the view admits moves the shader and the draw calls, and is
  its own change with its own pixels to prove; not built here.
