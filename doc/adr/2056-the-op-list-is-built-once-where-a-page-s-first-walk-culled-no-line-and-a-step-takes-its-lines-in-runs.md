# 2056 — The op list is built once where a page's first walk culled no line, and a step takes its lines in runs

Session 1610, slot 4 of batch ninety-six, `doc/todo/75` item 3 with item 2 brought back, on the project
owner's ask of 2026-10-19 that a zoom "send only a matrix" and the switch to code improvements of
2026-10-14. Status: **accepted** and **built**. Context: ADRs 0087, 1877, 1884, 1980, 1992, 1993, 2008,
2022, 2030, 2036; raster's ADR 0082; `doc/questions/A473`. Code: `raster-gpu`'s `encode/kept_lines.rs`
(`KeptRun`, `KeptLines`, `Before`, `Encoder::walk_kept`), `encode::encode_kept`, `retained::EncodeFrom`,
`Counters::lines_kept`; round 1593's page-space line lane and round 1600's `Ahead`
(`/home/AI/r1600-walk-false-sharing/line-lane-with-fix.patch`), now in the tree. Tests:
`tests/retained_replay.rs`'s `a_step_keeps_the_line_runs_and_draws_a_fresh_devices_frame`,
`a_line_inside_a_group_keeps_nothing`, `a_first_frame_that_culls_a_line_keeps_nothing`, and
`a_zoom_step_draws_the_line_stream_the_view_before_it_left` from a view the page fits. Instrument:
`zoom_frame`'s `lanes:` line, which ends in `lines kept`. Cites no clause and moves no ledger row: the
clauses the line lane reads (§8.4.3.2, §8.4.3.3, §10.7.5) are read as before, in the vertex stage.

## 1. The premise, and the hypothesis

**The premise held as ADRs 2008, 2022 and 2036 left it**: the step's op list was the view's because the
host's cull removed the lines the view did not admit, and the line instance was 40 of its 60 bytes the
view's; with the instance in the page's space and the cull in the vertex stage (item 2) every line is in
every view's list, so the list is a function of the scene. **The hypothesis held in its mechanism and
not in its population.** The view-independent part is the walk's lines, and taking them in runs takes
the line walk off the step: on the witness 4 398 932 lines in 1 055 runs, and the box each line grows
the page's plan by is never asked, because the page's medium already covers the target (section 2). But **"resident from the fit" holds only where there is a fit**: a page whose
first frame is a window — a turn at a magnification, a document opened zoomed — has no stream of the
page to keep, and item 2 alone made that frame send the page's 264 MB of lines where HEAD sent the
window's 26 (ADR 2036's 48 ms; this round's exploration, 50–80 ms of device time).

## 2. Decision

1. **A walk with no encode of the scene before it culls a line its view admits nothing of**, as HEAD's
   walk does, and writes the rest in the page's space (ADR 2022). A first frame in a window therefore
   sends what it shows; **a walk that culled no line wrote the page's whole stream**, and only that walk
   keeps runs.
2. **A kept run** is consecutive top-level commands that each wrote one closed-form instance, the
   instances consecutive in the stream, under one clip chain and one soft mask. A line inside a group or
   a soft mask, or a culled line, leaves the encode keeping nothing. **The line makers count the runs**
   (ADR 1884's threads, `count_runs`): each block's made lines carry the length of the run from them on,
   and the walk notes a block's runs once it has placed them, a run at a time — where every made line
   wrote its instance the block's instances are its made lines' in order. A note a line at a time on the
   walk's thread cost the witness's fit 15–20 ms of `encode` in an earlier build of this change; a walk
   no thread makes lines for (one encode thread, or a page below the makers' floor) notes each line.
3. **The encode of another view of the same scene** — the same device, lane and resource generation
   (`EncodeKey::replay_compatible`), a view that keeps the axes — walks every command outside the runs as
   the walk does and takes each run in one step: the replay records and the mask where its first line
   asks them, the chain resolved there, the queue drained, the lane counted, one `note_run` for its
   batch, and the stream **shared** (`Arc`) rather than written. A shared stream is the adoption key:
   the device's copy of it is the new encode's to the byte, so none of it crosses — which replaces
   ADR 2022's `LineRuns`, a second record of the same runs the walk paid for on every line.
4. **The box is asked only while it can still move the region.** Each line grows the page's plan by its
   ink at the new view inside its admitted rectangle, the arithmetic `Unclipped::of` uses
   (`line::device_ink`). The page's plan's box is read only as the region it renders into
   (`Region::of`, ADR 0036) — whole pixels clamped to the target — and a union of boxes inside the target
   cannot move that region once it is the whole target, so the lines after that are not asked and the
   region is the walk's. Past the fit the page's medium alone covers the target, and no line is asked.
5. **A run that is not this view's abandons the kept walk**, and the view is walked whole: a chain that
   now has a residue or another page rectangle, a line outside the runs that now takes the closed form,
   a run past the scene. None is reachable from a scene the handle owns at a view that keeps the axes;
   the check is the construction's, not a hope.
6. **A kept frame draws a fresh frame's pixels and not its counts.** A line the new view admits nothing
   of stays in the kept stream and the vertex stage collapses it, where a fresh walk would have culled
   it: `lanes.line` and `commands_culled` are the first walk's — item 2's change of bar, which the owner
   accepted in `A473` — and the frame's bytes are the fresh walk's, held on llvmpipe by the tests and on
   the Radeon 890M by `ZOOM_FRAME_DIGEST` (section 3).

## 3. Measured

One `--clock` hold (load 2.1–2.4, the Radeon 890M through RADV, headless, `Compute` coverage, 24 encode
threads), HEAD's export against the change's, interleaved, minimum of three rounds, two passes;
milliseconds from `zoom_frame`, the frame's total and in brackets its `encode`:

| the witness | HEAD | this change | line bytes, HEAD → change |
|---|---|---|---|
| the window 900 × 1100 at the fit (0.3775), first frame | 584.0, 584.8 (89.3, 91.4) | 590.2, 596.1 (95.3, 94.1) | 264 MB → 264 MB |
| then 2.4, the window's first frame past the fit | 122.6, 131.9 (92.1, 100.2) | 91.2, 93.4 (58.8, 60.6) | 26.3 MB → 0 |
| then 3.0 | 120.2, 125.2 (89.7, 95.1) | 88.4, 88.1 (56.8, 57.6) | 20.0 MB → 0 |
| then 3.6 | 117.6, 119.1 (92.5, 94.7) | 86.5, 87.4 (58.6, 60.0) | 14.2 MB → 0 |
| the window opened at 2.4, first frame | 358.3, 380.1 (86.9, 97.4) | 383.0, 370.5 (101.5, 99.9) | 26.3 MB → 26.3 MB |
| then 3.0 | 112.9, 118.5 (82.8, 88.7) | 124.3, 132.2 (94.2, 103.0) | the same |
| then 3.6 | 123.0, 121.8 (98.5, 97.6) | 119.8, 129.0 (93.3, 103.3) | the same |
| the page-sized target at the fit (0.594) | 607.6, 667.6 (113.3, 117.2) | 664.5, 616.5 (108.0, 106.2) | 264 MB → 264 MB |
| then 1.25× | 287.9, 295.2 (115.6, 125.6) | 215.6, 212.2 (73.9, 77.9) | 264 MB → 0 |

**The window past the fit is 30–38 ms faster a frame, its first frame included, and the page-sized
step 72–83 ms**; the walk's encode falls by about 35 ms and the transfer by 2–30. The window opened
at a magnification sends HEAD's bytes and walks HEAD's walk, culling as it does; its frames read
within this machine's spread of HEAD's, which an earlier hold of this round put the other way
(HEAD 129.8, 138.3 against 133.1, 120.3 at the 3.0 step). The fit itself is HEAD's within 3–11 ms. A
kept step's frame has the digest of the change's fresh frame at the same view on the Radeon (three
views), and the four device arms draw every page as HEAD's do (section 4).

## 4. The proof that nothing else moved

The six corpus arms' device four against HEAD's (`/home/AI/arms-1607/`): gpu and compute at 1× (968
pages each) and 4× (965), 0 pages differing and 0 one-sided — a corpus page is one frame, and a first
frame culls as HEAD's walk does. `pdf-model`'s `raster_golden` does not reach `raster-gpu` (`cargo tree
-i`). The turn gate's 33 figures: HEAD 33 inside; the change 32 inside and one at its band's floor,
`issue12841_reduced.pdf`'s step at 2.50 ms against 2.50–4.00, a page of two commands whose encode is
0.01 ms in both trees and whose other two children read 3.36 and 3.28 — no line, so nothing this change
can reach; the hold before read it at 2.96.

## 5. What is left

- **The cold window gets nothing yet**: its steps are walked whole and cull as HEAD's do, so they cost
  HEAD's. Keeping its lines needs the page's stream on the device without the first frame waiting for
  it — sent while nobody waits, the way ADR 1898 builds the scene ahead — which is the host's and is
  not built here.
- **The vertex stage pays for the lines a kept frame does not show**: 0–4 ms of `execute` a step in the
  window on the witness (18.7–18.9 against 15.7–16.3 at 2.4), the 4.4 million instances against the
  window's 0.24–0.44 million. Drawing a run through the
  indices the view admits (ADR 1980's `Window`, asked of the resident stream) is the lever, and it moves
  the shader, so it is its own change.
- **The record replay keeps no runs**: a page the record road replays (ADR 0087) walks its line records
  as before. The witness leaves that road at its 26th command; a replayable page of lines is the case.
- **What a kept step still walks is item 6's**: the strokes the line lane does not take, expanded on the
  host into the coverage sheet (8 MB a step on the witness), and its residue clip.
