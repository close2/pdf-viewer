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
draws, whatever its width. §8.5.3.2's round dot, a `m` and an `l` to one point, is one of these
marks on both backends (ADR 1857), unless no pixel of it holds a level; a single-point closed
subpath and a dot inside a longer path are still `split_degenerate`'s.

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
strips on the pool, the bounds kept with the list. The device's offscreen path takes the culled
list too: what the whole list moved on `bug946506.pdf` was raster's census sending a glyph placed
beyond the page to the atlas, not a pixel of the off-page marks (ADR 1858).

ADR 1817's candidate A: 4.0–4.5× at four times the fit, 2.2× at the fit, and a seam that moves
pixels (588 and 778 on the witness). A tiled processor entry in `render-cpu` is where it attaches;
what this file adds is the order — after section 1, so that what the pool multiplies is the small
cost — and the bin index that makes the cull proportional to the view (the linear one is 64–70 ms).

The toolkit windows do not reach the cull: they ask for the whole page at a zoom. At four times the
fit the whole page is 1 303 ms on four threads, the band of rows the window shows 444 ms and the
window 161 ms; the band needs a host to say it places a raster at a row offset, and the stand-in
and the look to know the band (ADR 1859 section 4). The strip count is the pool's width (ADR 1859).

## 4. The page drawn while it is read — built in `quorra` (ADR 1855)

The read was 76–78 % of the witness's first frame and the window showed a bar until it ended. Now
the interpreter hands its list over in parts as it builds it (`pdf_model::progress::Parts`), page
one is read on a thread of its own once the read has run `GLIMPSE_AFTER`, each part is drawn into
the first frame's target as it arrives (`render_cpu::Underway`), `quorra` shows each look while the
document opens, and the first frame is finished from that drawing — byte for byte the page drawn as
one strip, every corpus first page checked. ADR 1855 has the rule for each page that is drawn whole
instead and the figures. **Owed**: the two toolkit windows call `viewer_host::Ahead` (four calls in
each host, listed in ADR 1855 §5); `quorra --cpu`'s composer and a page turn after the launch.

## 5. A line kept on the device, the view a uniform — the build after

The device's zoom step on the witness is 1 168–1 196 ms (scene 399–417, encode 500–536, execute
108–153); the same frame replayed is 178 ms (ADR 1845 §4). Raster's line instances stated in the
scene's space and kept resident, the view's transform and §8.4.3.2's device-dependent thinnest width
applied in the shader, would make a step cost about the replay — vello's shape (`Scene::append`, its
flattening in compute). It needs the argument ADR 1835 §2 declined, that a scene holding a line is
true at every view; the lane rule that asks the device once on a page the seeds gave the processor
is built, with the seeds re-measured (ADR 1860).
