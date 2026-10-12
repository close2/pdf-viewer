# 1845 — The first frame is the read, and the next change draws the page while it is read

Session 1504, the step back of batch eighty on the project owner's target of 2026-10-10, "improve
performance" — the first frame, zooming in and out, the processor and the device, with a step back
every batch by first principles and by finding out how others do it. Status: **accepted**; it picks the
next build and builds nothing. Context: ADRs 1811, 1817, 1825, 1829, 1831, 1832, 1833, 1835, 1837;
`doc/todo/69`, `doc/todo/16`; Q393. Instruments: `crates/render-raster/examples/pipeline_price.rs`
(new), `examples/zoom_frame.rs`. Witness: `/tmp/demo_heavy_engineering_A1.pdf`. Cites no clause and
moves no ledger row.

## 1. Where the seconds go, after ADRs 1829, 1831, 1835 and 1837

HEAD `849a5cfc`, built from an export in its own target directory (`quorra` `08cb8fde…`), two clock
holds. **Headless**, pinned to the eight fast cores, eight rayon threads unless said, load 2.5–7.3:

| stage | figure | per unit |
|---|---|---|
| the open (file read, `Document::open`, the page found) | 5–41 ms | |
| the read, on a thread of its own | 2 575–2 652 ms (hold 1), 3 039–3 054 ms (hold 2) | 89–105 ns an operator |
| the read on a fast core / a slow core / unpinned | 2 590–2 608 / 3 769–3 881 / 3 018–3 161 ms | |
| the processor's fit draw (1416 × 1000 px), whole | 783–870 ms; 1 128 ms on one thread | 178–256 ns a command |
| the device's fit frame, `Cpu` coverage | 1 103 ms: scene 401, encode 505, transfer 50, execute 116 | 250 ns a command |
| the device's zoom steps (1.25×, 1.5625×, back to 1×) | 1 168, 1 196, 1 185 ms: scene 399–417, encode 500–536 | |
| the processor's zoom steps (page-sized targets) | 986, 1 046, 1 155, 1 018 ms | |

The `Compute` lane is slower at every step (1 134, then 1 626–2 399 ms). **In the windows**
(`quorra --trace`, ten `plus` notches 300 ms apart, two runs a display a hold), seconds from process
start; hold 2 ran at load 5.3 → 3.5, hold 1 at 5.7 → 15.7:

| | the read ends | first look | first frame | its draw | last notch to the real frame |
|---|---|---|---|---|---|
| owner's display, hold 2 | 3.61, 2.97 | 4.21, 3.50 | **4.78, 3.83** | 1.15, 0.85 | **1.50, 0.96** |
| owner's display, hold 1 | 4.05, 5.32 | 4.77, 6.09 | 5.49, 6.95 | 1.42, 1.61 | 2.17; the second run's keys reached no window |
| Xvfb, hold 2 | 3.38, 2.97 | 4.18, 3.69 | **4.31, 4.22** | 0.90, 1.23 | **1.03, 1.07** |
| Xvfb, hold 1 | 4.22, 6.72 | 5.25, 8.59 | 5.45, 9.72 | 1.18, 2.92 | 1.68, 5.64 |

Round 1497 read 6.73 and 5.20 s for the first frame and 1.43 and 2.11 s for the gesture on the
owner's display at load 9–13. **On the quiet sitting the read is 76–78 % of the first frame**, the
draw the rest; the window's read is the unpinned read headless, so nothing in the window adds to it
but the core it lands on, and the core is worth 45–50 % (slow against fast). A zoom notch is one
processor redraw, 0.90–1.24 s, plus the frame it takes back. The rest is small and was read off the
same traces: the device up in 6–36 ms on the owner's display (15–53 ms on Xvfb), beside the read and
not before it, and the processor's finished page uploaded and presented in 0.4–0.7 ms (4–13 ms on Xvfb).

**The brief's hypothesis half held.** The read is the largest term of the first frame. The scene
rebuild is not the largest of a zoom step: on the device encode is (500–536 ms against 399–417), and
the window draws the step on the processor, where there is no scene — the step is the whole redraw.

## 2. How others do it — evidence about approaches (principle 5)

- **pdf.js draws the operator list while it is still being built**: the worker's `OperatorList`
  flushes every `CHUNK_SIZE` (1 000) operators, and the main thread's `InternalRenderTask`
  continues on each chunk (`operatorListChanged` → `_continue`, `src/display/api.js`). poppler's
  `Gfx` paints through its output device operator by operator, and okular shows the part drawn every
  500 ms (ADR 1817 §2). mupdf without a display list (`fz_run_page` into the draw device) fuses the
  two the same way; with one, as here, it reads first.
- **None reads this page faster.** One fast core, read and drawn at 42.75 dpi: `mutool draw` 7.64 s,
  `pdftoppm` 14.33 s, against this tree's 3.77 s on one thread (2.64 + 1.13); `mutool -F stext` did
  not finish inside 180 s. Taken at load 14.6, beside a build — evidence of order, not a ratio.
- **hayro (under pdfcraft) keeps no list**: each tile re-interprets the content stream
  (`crates/render/src/raster.rs`), so its zoom pays the read. The list kept here is why a zoom never
  reads (ADR 1833 §5). For the device, vello 0.9 is the shape candidate B takes: a scene is encoded
  once in its own coordinates, `Scene::append` applies a view's transform to the encoding's transforms
  (linear in the scene, a copy), and flattening, binning and coverage run in its compute shaders
  (`flatten.wgsl`, `binning.wgsl`, `fine.wgsl`).

## 3. The candidates, by first principles

- **The read made faster in itself** has no order of magnitude in it. Lexing and the operand stack
  are 43 % of it (ADR 1833) and are the only part a split stream could share among cores — the
  operators' meaning is a function of every operator before them — so infinitely many cores give at
  most 1.8×; slot 5's dispatch cut is worth percents. The fast core against the slow one is worth
  more than any lever named, and it is the scheduler's.
- **Interpretation off the toolkit thread for a page turn** saves 0 s a zoom step and moves a turn's
  read off the thread without shortening it (ADR 1833 §5); candidate A below does it as a side effect.
- **A view-independent scene for lines** alone removes the scene, 399–417 ms of a 1 168–1 196 ms
  device step: 1.5×.
- **A. The page drawn while it is read**: the interpreter hands the list over in slices as it builds
  it and the processor paints each into one target as it arrives; the first look is the first slice.
- **B. A line kept on the device, the view a uniform**: raster's line instances stated in the
  scene's space and resident, the transform and §8.4.3.2's device-dependent thinnest width applied in
  the shader, so a zoom step encodes nothing for them.

## 4. The two priced, one measurement each

**A** (`pipeline_price` §6): the page read a second time while the first reading's 64 slices of
68 882 commands are drawn each as soon as the reader's figure has passed the operators that state
them — reader and drawer sharing the machine. Pinned, eight threads: serial 3 929 and 3 909 ms to the
finished frame, pipelined **3 464 and 3 390 ms** (−12 %, −13 %), the read slowed 11–13 % by the
drawer beside it; unpinned on the whole pool, 4 114 → **3 388 ms** (−18 %), the read slowed 1.8 %.
**The first slice is on the screen at 80–86 ms**, where today's first look waits for the read's end:
3.0–3.3 s headless, 3.50–4.21 s on the owner's display. In a window the first look is
`GLIMPSE_AFTER` (500 ms) into the draw, so the figure becomes about 0.6 s: **6–7× on the wait with
nothing on the screen**, 1.13–1.21× on the finished frame. One thread keeps up: its 64 slices sum to
1 169 ms against a 2 644 ms read.

**B** (`zoom_frame`, the fit frame asked for again at the same view, which raster replays): **178 ms**
(`Cpu`; execute 97, transfer 48 of 274 MB the replay still stages) against 1 168–1 196 ms for a
step that encodes; 186 ms against 1 626–2 399 on `Compute`. A replay is the floor B can reach, since
B leaves the device's execute where it is: **6.5×** on the device's step, about 9× if the instances
are not staged again. In a window it would need the lane rule to price the device on this page,
which ADR 1831 §4 says it never does (below).

## 5. Decision

**The next build is A: the page is drawn while it is read.** The interpreter publishes its list in
slices of top-level commands as it pushes them, beside ADR 1811's `Progress`; the window's processor
lane paints each slice into one target as it arrives, under ADR 1825's looks, and the finished list
is the one kept. Three reasons, in order: the open is the wait with nothing on the screen (3.5–4.2 s
of a bar), where a zoom already shows its stand-in at once (ADR 1832); A draws the same commands in
the same order into one target, so its pixels are the whole draw's by construction and the oracle
holds them byte for byte, where B changes the device's construction at every view and needs the
argument ADR 1835 §2 declined to make; and slot 1 of this batch builds the zoom's processor lever
(culled tiles), while nothing works on the first picture.

**B is the second build and is not dropped**: it is the only order of magnitude left on a zoom step
at the fit, where culling removes nothing. `doc/todo/69` §5 holds it.

What A's build must hold: the incremental target byte-identical to the whole draw on `raster_golden`
and the witness; a slice boundary only between top-level commands, never inside a group or a soft
mask; the device lane unchanged; the lane for a page whose command count is not known until its read
ends decided by a rule the build states (Q393's answer lets page one be the processor's); and the
concurrent figure of §4 retaken in the window on both displays.

## 6. Found beside it, not acted on here

- **The lane seeds are stale.** `Costs::DEVICE_SEED` is 1 788 ns a command; the device now draws the
  witness at 250. `PROCESSOR_SEED` is 667; the processor draws it at 178–256. The prediction (7.9 s
  against 2.9) still picks the faster lane on this page at the fit, and the device is never measured
  on a page the seeds gave the processor — so B, built, would never be used until that rule changes.
- **The read reads 2.59–3.05 s pinned here**, where round 1498's `display_list_census --stages`
  read 2.153 s. A third hold ran the two instruments interleaved on one fast core, three pairs:
  4 126–5 074 ms against 4 366–4 505 ms, the minima 6 % apart — at load 15–28, so the difference is
  the sitting's and not the instrument's, and a quiet figure is still owed to slot 5's baseline.
