# 2008 — The op list is a function of the view through the host's cull, so it is built once only after the line lane maps on the device

Session 1586, a "pixels" round of batch ninety-three, on `doc/todo/75` item 3 for the project
owner's ask of 2026-10-19 that a zoom "send only a matrix". Status: **accepted** — a measurement
and an ordering, nothing built under it; item 3 stays after item 2, which waits on
`doc/questions/Q473`. Context: ADRs 0087, 1877, 1884, 1992, 1993; raster's ADR 0097. Instrument:
`render-raster`'s `examples/zoom_frame.rs` (its `crossed:` and `lanes:` lines and `culled`
column) and one scratch probe of `raster-gpu`'s replay road, never committed. Cites no clause and
moves no ledger row; no pixel moves.

## 1. The premise, and why it did not hold

The brief read item 3 as building the device's op list once per page and re-encoding only the
view-dependent remainder, "9 474 of 4 408 406 commands on the witness". **That figure is the
remainder once item 2 has made the line batches view-free**, and `doc/todo/75` item 3 says so in
its first sentence ("[a] retained encode whose line batches are view-free replays at a new view by
writing the uniform"). Today three things tie every line's output to the view:

- **The instance.** A line's sixty bytes (`encode/instance.rs`, `line_head` and
  `LineInstance::under`) are fifteen floats: the two device endpoints, the pen's two device
  semi-axes, Table 53's end, the premultiplied colour and the admitted rectangle. The end and the
  colour, 20 bytes, are the page's; the other 40 are the view's.
- **The op list itself.** A line the host culls writes no instance, so the ranges every
  `Op::Draw` batch names move with the cull. `zoom_frame` on the witness in the shipped window
  (`ZOOM_FRAME_WINDOW=900x1100`, `Compute`, scale 2.4 then ×1.25 and ×1.5, HEAD): the line lane
  admitted **438 940, 332 995 and 235 905** lines, the cull dropped 3 968 567, 4 074 668 and
  4 171 937 commands, and the step at ×1.25 left 105 945 lines (24 %) out of a list kept from the
  view before. At a page-sized target (fit 0.594, then 1.25×) nothing culls and the list's shape
  holds, which is the one case where a kept list would be the same list — and the window takes
  over exactly past it.
- **The road that would carry a kept list.** Record replay (ADR 0087) is the mechanism by which a
  retained encode is re-placed at another viewport; the probe shows the witness leaves it at its
  twenty-sixth command, for a residue clip (`encode/fill.rs`, `unreplayable`), with 25 records of
  112 bytes written. Every step of the witness is the full walk, under `Compute` as under `Cpu`.

**The hypothesis — the op list's view-independent part is nine tenths of the encode — is
therefore not true of the encode this tree runs.** HEAD's step at 1.25× (instrumented, one run,
load 5.7): encode 179.1 ms, of which the walk (`encode: recording`) 137.5 and the forms made ahead
(`geometry`) 36.4; the walk's 280-odd instructions a line (ADR 1884 §3) are the chain, the cull,
the instance and the plan, each of which reads the view through one of the three items above.

## 2. Decision

**Item 3 is built after item 2, not beside it.** Item 2 moves §8.4.3.2's zero width, §10.7.5's
adjustment and the cull into `line.wgsl`'s vertex stage, so an instance outside the admitted
rectangle collapses its quad on the device instead of leaving the list on the host: that is what
makes the batches' ranges a function of the page alone, and only then is the op list a thing that
can be built once. A kept list built before it would either fall back on every window step (the
24 % above) or keep culled lines as instances, which changes `bytes_uploaded` and
`commands_culled` — item 2's own change of bar, which the owner is asked in `Q473` and which no
round may take ahead of the answer.

**What item 3 also owes, found here**: the witness's first residue clip takes the frame off the
record road at command 25, so a kept list on the witness cannot ride ADR 0087's replay as it is.
Either the kept list is held apart from the record list (the lines' ranges keyed by the scene
alone, the remainder re-walked), or a residue clip's frame-wide rasterisation is made replayable;
`doc/todo/75` item 3 names both, unpriced.

## 3. What was built instead

The brief's second half: `render_raster::FrameCost::streams` carries raster's `UploadSplit` and the
window's frame line prints it (ADR 2009), so the owner's trace says what each frame sent — the
instrument item 2 and item 3 will each be read against when they are built.
