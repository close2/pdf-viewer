# A mark sized before it becomes geometry: the increments ADR 1817 picked

Status: **open — sections 1, 2 and 3 built (ADRs 1829, 1835, 1841), sections 4 and 5 not; section 4 is the
next build (ADR 1845).** ADR 1817 argues the file and prices it against the other candidate, ADR 1845
re-takes the pipeline after sections 1 and 2 and picks between sections 4 and 5; this file is the
work, in the order it is to be taken.
Priority: 40-band — performance, measured and priced; the band was full when this was written.
Witness: `/tmp/demo_heavy_engineering_A1.pdf`, the owner's, **not in the repository** — 4 408 405
marks, 99.0 % of them under a quarter pixel at the window's fit; the deterministic in-tree generator
of its shape — cubicle grids, hatching, labels — is `crates/render-raster/examples/heavy_drawing.rs`
(ADR 1823), and a gate that holds its ladder is still owed.
Instrument: `crates/render-raster/examples/tile_price.rs` — its census, its fit row, its floor row;
`examples/pipeline_price.rs` for the read against the draw (section 4).
Clauses: §8.4.3.3 (Table 53's caps), §8.5.3.2 (a stroke's shape), §10.7.4 (the area a shape paints).
Code: `crates/render-cpu/src/lib.rs` (`draw_stroke`, the place the classification goes before
`convert::path`), `crates/pdf-render/` (the classification itself, one copy for every backend —
trap 2), `raster/crates/raster-gpu/src/` (the device primitive and its per-mark price).

## 1. The processor's kernel — the first increment

A single straight segment, stroked with any of Table 53's three caps, no dash, a solid paint and
§11.3.5's `Normal` blend, under a transform that keeps the axes, whose ink lies within a 2 × 2
block of device pixels: its exact area in each pixel of the block is computed in closed form — a
rectangle's overlap is separable, a round cap's is a disc's segment — and composited once, with
the clip and the soft mask multiplied as every other mark's are. Nothing of `tiny-skia` is entered.
That population is the census's whole witness population; everything outside it takes today's path.

**Built** (ADR 1829): `pdf_render::thin_segment` admits the segment in any direction — the axis-only
first cut left the witness's 667 904 hatching dots on the general path — and
`render_cpu::thin_segment::draw` paints it. The fit raster on one thread went from 2.9 s to 0.84 s,
not the 0.46 s priced, and what is left is the command loop: every strip replays every command's
extent test. Two things are still owed. `drawn_row_by_row` may let a cut pass a stroke this kernel
draws, whatever its width. A zero-length round dot is a disc this construction already measures.

**Owed beside it**: a unit ladder of the kernel against the clause's closed form — sizes 0.05 to
2 px, positions across pixel edges, all three caps, both axes — whose expected values are the
areas §8.4.3.3 and §10.7.4 define, never the old path's; a control that must stay on the general
path (a two-segment stroke, a dashed one, a turned one); `raster_golden` and its division test with
every moved digest named against ADR 1742; `tile_price`'s fit row and `tools/state.sh heavy`
before and after, as clock walks. Priced at 7.3 s → about 0.46 s on the witness's fit raster at
three times the measured floor (ADR 1817 section 3).

## 2. The same classification on the device

Raster gains a primitive for the same marks, its coverage computed analytically on the device and
its budget counted per mark — the witness is charged 96 B a command today by the encoder's
pre-count of one rect and one quad instance (ADR 1823), which the 2 GiB frame budget admits whole;
what the primitive removes is the resident outline per one-segment stroke, the 512 MiB resource
budget's ceiling (ADR 1823's chop 16), and the host's flattening of every such stroke. Byte agreement with the processor's kernel is the oracle's question and is
stated as raster's ADR 0082 tolerance, not assumed.

**Built** (ADR 1835, raster's ADR 0097): `render-raster` sends every stroke `thin_segment` admits as
one `Command::Line`, which raster draws from its exact area in the shader — any direction, under a
transform that keeps the axes — with no outline behind it, 60 B an instance and 64 B of pre-count.
The witness's device frame at the fit went from 9.7 s to 1.2 s on both lanes, and the generator is
drawn at chop 24 (12 185 819 commands), where HEAD's resource budget refused chop 12. Still owed: a
scene holding a line is rebuilt at every view (ADR 0702), so a zoom step pays the scene again.

## 3. Tiles from a culled list, on a pool — the multiplier

**Built** (ADR 1841): a zoomed frame is drawn from the marks that reach it, as the window's own row
strips on the pool, the bounds kept with the list; the device's offscreen path keeps the whole list.

ADR 1817's candidate A: 4.0–4.5× at four times the fit, 2.2× at the fit, and a seam that moves
pixels (588 and 778 on the witness). A tiled processor entry in `render-cpu` is where it attaches;
what this file adds is the order — after section 1, so that what the pool multiplies is the small
cost — and the bin index that makes the cull proportional to the view (the linear one is 64–70 ms).

## 4. The page drawn while it is read — the next build

After sections 1 and 2 the read is 76–78 % of the witness's first frame (owner's display, 2.97–3.61 s
of 3.83–4.78 s) and the window shows a bar until it ends. The read itself has no order of magnitude
left (ADR 1845 §3), so the change is not to wait for it: the interpreter publishes its list in slices
of top-level commands as it pushes them, beside ADR 1811's `Progress`, and the processor lane paints
each slice into one target as it arrives, under ADR 1825's looks — pdf.js's streamed operator list,
okular's partial updates. Priced by `crates/render-raster/examples/pipeline_price.rs` §6, the
reader and the drawer at once: the first slice on the screen at 80–86 ms where the first look waits
3.0–3.3 s, the finished frame 1.13–1.21× sooner. **Owed with it**: the incremental target
byte-identical to the whole draw on `raster_golden` and the witness; slice boundaries only between
top-level commands; the lane rule for a page whose command count is not known until its read ends;
the first look and first frame re-taken on both displays. `doc/todo/16`'s resumable interpreter is
not needed for it: the reader runs to its end on its own thread, as it does today.

## 5. A line kept on the device, the view a uniform — the build after

The device's zoom step on the witness is 1 168–1 196 ms (scene 399–417, encode 500–536, execute
108–153); the same frame replayed is 178 ms (ADR 1845 §4). Raster's line instances stated in the
scene's space and kept resident, the view's transform and §8.4.3.2's device-dependent thinnest width
applied in the shader, would make a step cost about the replay — vello's shape (`Scene::append`, its
flattening in compute). It needs the argument ADR 1835 §2 declined, that a scene holding a line is
true at every view, and the lane rule to measure the device on a page the seeds gave the processor
(ADR 1831 §4), whose seeds are stale (ADR 1845 §6).
