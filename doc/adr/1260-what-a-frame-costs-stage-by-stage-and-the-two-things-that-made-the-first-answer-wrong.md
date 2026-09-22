# 1260 — What a frame costs, stage by stage, and the two things that made the first answer wrong

Session 1211. Status: **accepted**. Adds `crates/render-raster/examples/frame_budget.rs` and
`tools/state.sh frame`. Amends no earlier measurement: every figure in
[ADR 0767](0767-a-zoom-step-on-the-lane-the-window-takes.md) and `doc/todo/47` is a *zoom step* on
the compute lane and stands.

## 1. The question nothing here could answer

`doc/todo/36` asks for a picture every refresh — 60 Hz as the floor, 120 Hz as the target, so
**8.333 ms**. Three instruments measured parts of that. `examples/zoom_frame` splits a
magnification into raster's phases and begins from a display list somebody else built.
`crates/viewer-ui/tests/launch_path.rs` times a page turn end to end and reports one number.
`quorra --trace` reports a cadence but not what a frame is made of. **None of them says what share
of a refresh each stage of a page turn takes**, and the first two cannot be added, for the reason
in §2.

`frame_budget` is that instrument. Three page classes — a dense text page, a page of §8.7.4.5.7
patch meshes, a page that is one five-megapixel photograph — at 1× and at 2×, each in three rows:
the page **turned** to (interpreted and drawn on a device that has already drawn another
document's page), the same frame asked for again (**warm**), and the page placed at twice the
magnification (**step**). Every row is the minimum of several rounds, each on a device of its own,
with the load average printed either side.

## 2. The first thing that made it wrong: a page turn and a zoom step are different rasterisers

`viewer-ui`'s `lane_for` hands a *moved view* `Coverage::Compute` and everything else the lane
`coverage_for` picks from the magnification, which below 10× is `Coverage::Cpu` (ADR 0700). So the
window turns a page with one of raster's rasterisers and zooms with another, and a budget for one
measured on the other is a budget for a configuration nobody runs. `doc/todo/47` records this
mistake in the other direction — a resize step read at 129 ms of `encode` on the lane the example
defaulted to, against 9.4–10.1 on the lane the window takes. This example takes the lane from the
row: `Cpu` for the turn and the warm repaint, `Compute` for the step.

## 3. The second thing: the font cache a page turn actually has

The first table this round took put the text page's interpretation at **12.45 ms**, which read as
the largest stage of a page turn and very nearly became this round's subject. It was measuring
`pdf_model::content::interpret`, which builds a `FontCache` of its own; `viewer-core` keeps one for
as long as the document is open and hands it to every page, so §9.6's font programs are loaded once
per *document*. Interpreting the preceding page first, against the same cache — which is what
`Command::GoTo(Next)` has — puts the same figure at **1.37 ms**. A factor of nine, in the stage the
question was about, from a cache the instrument did not model.

The general form is `doc/habits/measuring.md`'s: a price is a claim about a configuration, and the
configuration a gesture runs in includes what the *last* gesture left behind.

## 4. What a frame is made of

890M through RADV, 1 600 × 1 000 window, minimum of five rounds, load average 2.7. Milliseconds,
and the percentage is of one 120 Hz refresh. `budget` leaves out the readback, which this example
pays and a window does not.

| page | row | budget | interp | scene | encode | transfer | elsewhere | execute |
|---|---|---|---|---|---|---|---|---|
| ISO 32000-2 p101 — text, 3 007 commands | turn | 9.25 (111%) | 1.37 | 0.44 | **6.38** | 0.22 | 0.52 | 0.32 |
| | warm | 0.35 (4%) | — | — | 0.00 | 0.02 | 0.25 | 0.08 |
| | step | 1.47 (18%) | — | — | 0.22 | 0.11 | 1.03 | 0.11 |
| `personwithdog.pdf` p1 — meshes, 18 commands | turn | 10.75 (129%) | 3.69 | 2.73 | 1.48 | 0.55 | 1.95 | 0.35 |
| | warm | 0.86 (10%) | — | — | 0.00 | 0.04 | 0.61 | 0.21 |
| | step | 11.18 (134%) | — | **3.98** | 4.21 | 0.54 | 2.06 | 0.39 |
| `issue12841_reduced.pdf` p1 — one photograph | turn | 131.58 (1579%) | **78.68** | 0.01 | 0.01 | **51.49** | 1.20 | 0.19 |
| | warm | 0.24 (3%) | — | — | 0.00 | 0.00 | 0.15 | 0.08 |
| | step | 11.14 (134%) | — | 0.00 | 0.01 | **8.23** | 2.72 | 0.18 |

Four things this says that nothing in this tree said before:

- **A repaint of what is already on the screen always fits**, on every page class: 3% to 10% of a
  refresh. Whatever else is true, a window that has drawn its page can answer a selection change or
  a chrome change at 120 Hz.
- **A page turn onto a page whose resources the device has not seen fits on none of them.** The
  text page is 111% of a refresh. That is the expensive end of the gesture and the row says so:
  the warm-up is another document, so all 207 outlines are uploaded inside the timed frame.
  `launch_path`'s `turn_ms` — five arrow keys inside one document whose page one is drawn, where a
  glyph outline is an `Arc` the cache already holds — is the other end, and a session moves
  between them.
- **The largest stage is a different one per page class**, which is why one witness could never
  have answered this: raster's encode for text, this tree's own scene walk and interpretation for
  vector artwork, and interpretation plus the transfer for a photograph.
- **`execute` — the graphics device's own passes — is 1% to 5% of every row.** ADR 0387 measured
  0.07% on one page and this is the same finding across three classes: a slow frame here is a host
  thread, and there is no lever on the device.

## 5. What was taken, and what was written down instead

The largest stage that is this tree's own and that a round could reduce was the mesh page's scene
walk, 8.22 ms of an 18.04 ms zoom step. ADR 1259 divides it and it is 3.98. The two larger costs
are raster's — the CPU-lane encode of a page seen for the first time (6.38 ms, 77% of a refresh)
and an image restaged per placement (20 MB on the turn, 80 MB on the step) — and they are
`doc/QUORRA_FEEDBACK.md` §52 with the table above under them.

The one remaining large cost that is this tree's is the photograph's 78.68 ms of interpretation.
Callgrind attributes 65% of it to `zune-jpeg`'s own decode and about 17% to two functions in
`pdf-model`'s `image.rs` — the expansion of decoded components into RGBA, and a linear scan of the
whole codestream looking for a `DNL` marker that nearly no JPEG has. That file belongs to no round
this batch; `doc/todo/36` records it with its numbers so the next round does not have to find it
again.

## 6. What this instrument cannot see

The **present**. The presenter owns the surface on the event thread (ADR 0391) and there is no
surface without a display server, so `frame_budget` is the launch path's own omission in another
place: everything up to the frame being finished, and nothing about putting it on the screen.
`quorra --trace`'s cadence summary remains the only instrument for that half, and
`doc/environment.md`'s `Xvfb` recipe the only way to run it here.
