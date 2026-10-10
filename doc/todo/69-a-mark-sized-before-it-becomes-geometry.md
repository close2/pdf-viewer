# A mark sized before it becomes geometry: the increments ADR 1817 picked

Status: **open — chosen, nothing built.** ADR 1817 argues it and prices it against the other
candidate; this file is the work, in the order it is to be taken.
Priority: 40-band — performance, measured and priced; the band was full when this was written.
Witness: `/tmp/demo_heavy_engineering_A1.pdf`, the owner's, **not in the repository** — 4 408 405
marks, 99.0 % of them under a quarter pixel at the window's fit; a deterministic in-tree generator
of its shape — cubicle grids, hatching, labels — is what a gate can hold, and the gate owes one.
Instrument: `crates/render-raster/examples/tile_price.rs` — its census, its fit row, its floor row.
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

## 3. Tiles from a culled list, on a pool — the multiplier

ADR 1817's candidate A: 4.0–4.5× at four times the fit, 2.2× at the fit, and a seam that moves
pixels (588 and 778 on the witness). A tiled processor entry in `render-cpu` is where it attaches;
what this file adds is the order — after section 1, so that what the pool multiplies is the small
cost — and the bin index that makes the cull proportional to the view (the linear one is 64–70 ms).

## 4. Interpretation, the next floor

2.3–2.6 s on the witness and, once section 1 is built, the largest term of its first frame. Not
priced: drawing the list's prefix while the rest is read (okular's partial updates) is the shape to
price first, against `doc/todo/16`'s resumable interpreter.
