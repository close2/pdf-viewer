# RFC 0013 — The game-like lane: a page resident as triangles on the hardware rasteriser, priced

Status: **proposed** — ready for the owner. Nothing in this document changes a ledger status, a
lane, a gate or a line of code that ships. The round that wrote it built one probe, an example
binary (`crates/render-raster/examples/resident_triangles/`), and measured it beside the compute
lane on the documents ADR 2408 measured.
Round: 1796, a "pixels" round added to batch 119 at the owner's word of 2026-10-26.
Builds on: ADR 0368 (the tessellation lane refused), ADR 0698 (the compute rasteriser priced), ADR
0702 (the scene in page space), ADRs 2315 and 2355 (the zoom that re-encodes nothing, the compute
lane's first frame), ADR 2408 (the blurry frame on a zoom step); raster's ADRs 0016 and 0076 (its
GPU coverage lane and the clause it does not meet), 0079 and 0080 (the compute lane and its
determinism), 0091 and 0095; `doc/todo/75`.
Decision record: ADR 2426. Asks: **Q662** (section 8).

`§N` is a clause of ISO 32000-2. *MSAA* is multisample anti-aliasing: the hardware decides, at
`n` fixed points inside each pixel, whether a triangle covers it, and the resolve averages them.

---

## 0. In one paragraph

The owner's lane exists and is fast. A page's fills become triangles once — a fan over each
subpath's chords, one Loop–Blinn triangle per quadratic piece whose inside the fragment shader
decides per sample, and one covering rectangle that paints where the stencil's winding count
passes §8.5.3.3's rule — recorded once as a render bundle, with a zoom step writing one 32-byte
uniform. On the Radeon 890M at 8× MSAA a step of ISO 32000-2 p. 101 costs **0.87 to 1.15 ms**
where the compute lane's costs **1.2 to 2.6 ms**; the two-page column the compute lane cannot keep
in page space costs **1.27 to 1.49 ms**; and the heavy plan sheet, whose weight is 52 million image
samples rather than its paths, costs **1.30 to 2.38 ms** against **5.6 to 39.7 ms**. What it
gives up is exactly what the compute lane was built to keep: coverage is a count of eight samples,
not §8.5.3.3's area, so 11 to 14 % of the pixels the oracle inks on a text page at 1× receive no
ink, a path loses §10.7.4's guarantee that no shape disappears (a glyph is exempt from it by the
clause's own last paragraph), the bytes differ between adapters, and transparency groups, soft
masks and every blend mode but Normal need layers drawn per view. **PDF's paint is not
incompatible with the device's painting**: fills, both winding rules, curves, glyphs, solid and
constant-alpha paint, images and shadings each map onto a hardware primitive; what does not is
exact area coverage, the no-disappearance rule for paths, compositing that reads its backdrop, and
byte-identical output.

## 1. The question

The owner, 2026-10-26, verbatim: "but why don't we send everything to the GPU in a way, that it's
GPU elements?  Games also don't need to send everything again, whenever something zooms?
(rastering first is of course not what I mean)." and "or are the PDF paint instructions completely
incompatible to GPU painting instructions?"

## 2. What the tree already does, and what was never priced

- **The compute lane** (raster's ADR 0080, this tree's ADR 0698) is already resident: a page's
  outlines live in a segment arena on the device, and a zoom step replays the encode's records
  (`record-replayed`) and recomputes exact coverage — signed trapezoid areas, 256 levels, the CPU
  oracle's bytes (raster's ADR 0079) — in compute passes. It does not "send everything again":
  on ISO p. 101 a step crossed 7.0 to 8.4 MB in this round's runs, the per-view instance and
  record streams, against the probe's 32 bytes.
- **raster's `Coverage::Gpu`** (raster's ADRs 0016, 0076) already rasterises outline triangles with
  Loop–Blinn curve triangles, but into coverage *tiles* of an atlas, with Wallace's float-target
  winding, and composites the tiles; ADR 0368 refused it as a tessellation lane whose vertices
  outweigh a small tile.
- **Never priced**: the lane a game uses — the whole page's triangles drawn straight into the
  frame by the hardware rasteriser under MSAA, winding counted in the stencil buffer, the view a
  uniform, the command stream recorded once.

## 3. The lane as the probe builds it

- **Fills** (§8.5.3.3): stencil-then-cover (Kokojima et al., 2006). The fan and curve triangles
  increment the stencil through a front face and decrement through a back face (nonzero), or
  invert it (even-odd); the cover rectangle tests it, paints, and zeroes it. Consecutive marks of
  one paint and one rule whose rectangles do not meet share one stencil and one cover pass, which
  draws them exactly as one pass each would: ISO p. 101's 3 005 fills are 94 such batches.
- **Curves**: §8.5.2.2's cubics are cut into quadratics once, at a page-space tolerance of 1/64
  device pixel at 64× the fit; each quadratic's triangle keeps the samples where `u² − v < 0`,
  evaluated per sample. Cubic Loop–Blinn (classification into serpentine, loop and cusp) would
  avoid the cut and is not what the probe prices.
- **Strokes** (§8.5.3.2): dashed in path space and outlined once by `kurbo`, as `render-raster`
  already does per view, at the same page-space tolerance, then filled nonzero. A zero-width
  stroke is not carried: §8.4.3.2's thinnest line is a width in device pixels.
- **Images** (§8.9.4): a textured quad over the unit square, magnified nearest-sample unless
  `/Interpolate` (§8.9.5.3), minified linearly from the full grid with no mip chain.
- **Groups** whose every parameter is the default are walked as runs; the plan sheet's ten knockout
  groups were walked as runs to price their geometry, which §11.4.6 does not permit (section 5).
- **Clips** are not carried: the probe draws a clipped mark unclipped, and so does the oracle it is
  compared with. ISO p. 101 clips 2 900 of its 3 005 fills; a clip costs a stencil bit and a pass
  per nesting level that this round did not price.

## 4. The figures

All on the Radeon 890M (`AMD Radeon 890M Graphics (RADV STRIX1)`, radv Mesa 26.2.4), headless,
release, an 800 × 1600 window (ADR 2408's), HEAD's build plus the probe, one `--clock --nice 0`
hold of 105 s: two interleaved passes, every probe figure the minimum of 7 round-robin rounds, every
compute-lane figure `examples/zoom_frame.rs`'s with `ZOOM_FRAME_COVERAGE=compute`, the minimum of
3, its cold first frame dropped. The one-minute load was 2.92 at every arm's start but the first's
(4.18, after the 60 s cap); `pp_dpm_sclk` stood at its middle line, 1 093 to 1 648 MHz, before and
after every arm; no other process of this account held the render node. The probe's figure is a
step's host time plus its wait to completion; the compute lane's is its `device` less `readback`,
since a window never reads back. Milliseconds, the range over both passes:

| page | 0.5× | 1× | 1.25× | 2× | 4× | 8× | 16× |
|---|---|---|---|---|---|---|---|
| `issue19802` p. 1, probe 8× | 0.44–0.46 | 0.62–0.67 | 0.68–0.74 | 0.92–0.94 | 1.31–1.39 | 1.35–1.37 | 1.41–1.42 |
| `issue19802` p. 1, compute | 3.2–3.3 | 1.4–1.5 | 1.4 | 1.4–1.5 | 1.3–1.5 | 1.3–1.4 | 1.4–1.5 |
| ISO p. 101, probe 8× | 0.87–0.89 | 0.95 | 1.06–1.07 | 1.11–1.15 | 1.07–1.10 | 1.09–1.13 | 0.96–0.97 |
| ISO p. 101, probe 4× | 0.70–0.75 | 0.72–0.80 | 0.79–0.84 | 0.78–0.86 | 0.71–0.78 | 0.73–0.79 | 0.69–0.73 |
| ISO p. 101, compute | 2.4–3.2 | 2.3–2.5 | 2.3–2.6 | 2.0–2.1 | 1.4–1.5 | 1.4–1.5 | 1.2–1.6 |
| ISO pp. 101–102 column, probe 8× | 1.27–1.29 | 1.29–1.30 | 1.37–1.39 | 1.48–1.49 | 1.39–1.41 | 1.45–1.46 | 1.30–1.31 |
| `22060_A1_01_Plans` p. 1, probe 8× | 1.42–1.43 | 2.13–2.16 | 2.33–2.38 | 1.72–1.73 | 1.35–1.40 | 1.30–1.37 | 1.41–1.47 |
| `22060_A1_01_Plans` p. 1, compute | 12.6–16.0 | 11.7–11.8 | 19.1–19.2 | 14.4–14.7 | 38.8–39.7 | 8.7–8.8 | 5.6–6.1 |

`zoom_frame` draws one page, so the column has no compute-lane row here; in the window ADR 2408
measured its device time at 4.8 to 9.9 ms, every step rebuilding its scene.

**Once, per page**: ISO p. 101 is 268 122 fan, 215 058 curve and 18 042 cover vertices, 10.0 MB,
tessellated in 8.5 to 9.4 ms and uploaded in 2.0 to 2.7 ms; the column 19.2 MB in 14.3 ms and 3.8
ms; the plan sheet 0.4 MB of paths and 52 254 938 image samples, 209 MB of texture, uploaded in
55.5 ms. Pipelines compiled in 1.0 to 15.0 ms, the bundle recorded in 0.1 ms; a step's host time
is 0.03 to 0.38 ms, the bundle's replay. The vertices are pre-transformed per placement; instancing
one outline per distinct glyph would divide a text page's bytes by its repetition.

**What the hypothesis said, and what held.** "Fills at 8× MSAA cost a fraction of the compute
lane's per step on the dense and heavy pages": on the dense page the probe's step is 0.37 to 0.80
of the compute lane's, which is a fraction but not an order; on the heavy page the probe is 4 to 28
times cheaper, **but the heavy page is not a page of fills**: its 24 fills and 40 strokes are
nothing beside its 16 images, and the compute lane's step there is the images' samples crossing
again (its `transfer` reached 29.0 ms at 4×). The lever the heavy page names is an image resident
as a texture, which is a lever of its own (section 7).

## 5. What it gives up — the hard bars

Each is measured where the probe could measure it, against `render-cpu` drawing exactly the marks
the probe carried, each without its clip, at the same views (`RESIDENT_ORACLE`).

1. **Exact area coverage.** §8.5.3.3 paints "the entire region enclosed by the current path"; MSAA
   paints a count of 8 points, 9 levels, where the oracle paints 256. On the text pages the
   probe stays inside the corpus gate's three bounds (`crates/render-raster/tests/corpus.rs`:
   mean below 1.5, worst 32-pixel tile below 7.0, structural similarity above 0.99) at every rung:
   ISO p. 101 mean 0.08 to 0.77, worst tile 0.88 to 4.11, similarity 0.9976 to 0.9997; but 16 to
   29 % of its differing pixels differ by more than 16 levels, the worst by 57 to 87 (57 to 123 on
   `issue19802`).
2. **§10.7.4's no-disappearance.** "A shape shall be scan-converted by painting any pixel whose
   half-open square region intersects the shape, no matter how small the intersection is." At 1×
   the probe left white 14 342 of the 125 079 pixels the oracle inks on ISO p. 101, and 3 521 of
   25 793 on `issue19802`: a pixel whose share of a mark misses every sample gets nothing. For text
   that is permitted — "Scan conversion of character glyphs may be performed by a different
   algorithm from the preceding one" — and for a path it is not: a hairline narrower than the
   sample spacing can vanish, which is raster's ADR 0076 non-conformance at 8 samples instead of 16.
   A remedy exists (drawing every path's boundary as conservative-rasterised lines, or routing thin
   marks to the compute lane as raster's ADR 0070 routes them to the CPU lane), and it is a cost
   the probe did not take.
3. **Even-odd and nonzero over self-intersecting paths: not a bar.** The stencil counts §8.5.3.3.2's
   crossings exactly per sample, and inverts for §8.5.3.3.3; the column's 42 even-odd fills draw
   as the rule says. The one limit is the stencil's eight bits: a winding number that is a nonzero
   multiple of 256 reads as outside.
4. **Strokes.** Joins, caps, miters and dashes are outlined once in page space and are no bar. Two
   things are a width in device pixels and so belong to the view: §8.4.3.2's zero-width line and
   §10.7.5's stroke adjustment; a lane would tessellate those per view, or draw them as the compute
   lane does.
5. **Images.** §10.7.4 makes an image's rule the device's own: "only those pixels whose centres lie
   within the region shall be painted ... There shall not be averaging over the pixel area." That
   is the hardware rasteriser's rule and a nearest sample, exactly. The tree's choice to average a
   minified image (one of the three decisions raster's `CLAUDE.md` names as settled upstream) is not, and a mip chain only
   approximates it: the plan sheet at 1× is outside every corpus bound (mean 3.32, worst tile
   23.48, similarity 0.8804) because its minified scans alias, and inside them at 16×.
6. **Transparency groups and soft masks** (§11.4, §11.6.5). An isolated or knockout group, a soft
   mask, and every blend mode the fixed-function blender lacks read the group's own backdrop, so
   each needs an offscreen layer drawn at the view's size on every step: the price scales with the
   group's area in pixels, and knockout (§11.4.6) composites each element against the group's
   initial backdrop rather than the one before it, which no blend state expresses. The plan
   sheet's ten knockout groups were drawn here as runs, which is wrong where their elements
   overlap.
7. **Byte-determinism across adapters.** The compute lane's bytes are the CPU lane's on every
   adapter (raster's ADR 0079). The probe's are not: RADV and llvmpipe drew ISO p. 101 at 8× with
   different digests, 385 pixels apart at 1× and 162 at 4×, each by 32 levels — one sample of
   eight. Sub-pixel precision and sample placement are the driver's, and WebGPU guarantees only
   1 and 4 samples (8 is an adapter feature this round had to ask for).

## 6. The owner's second question, answered plainly

No — PDF's paint instructions are not incompatible with the device's. Most map onto a primitive the
hardware already has; the exceptions are the ones the specification defines by area or by backdrop.

| PDF construct | onto the hardware | how |
|---|---|---|
| a fill, nonzero or even-odd (§8.5.3.3) | yes | stencil-then-cover, exact per sample |
| a Bézier curve (§8.5.2.2) | yes | Loop–Blinn per fragment, after a one-time cut or by cubic classification |
| a glyph (§9.2, §10.7.4's last paragraph) | yes | a fill; the clause lets glyphs be scan-converted differently |
| a stroke's joins, caps, dashes (§8.4.3) | yes | outlined once in page space, then a fill |
| a zero-width stroke, stroke adjustment (§8.4.3.2, §10.7.5) | per view | a width in device pixels |
| a solid or constant-alpha paint, Normal blend (§11.3) | yes | the fixed-function blender |
| an image (§8.9, §10.7.4) | yes | a textured quad; nearest sample is the clause's own rule |
| an image minified by area averaging (this tree's choice) | approximately | a mip chain, or the compute lane |
| a shading (§8.7.4.5) | yes | a fragment program; raster already compiles §7.10's functions to WGSL |
| a clip (§8.5.4) | yes | stencil bits, one pass per nesting level |
| a blend mode other than Normal (§11.3.5) | no, beyond a few | reads the backdrop: a layer or a copy per mark |
| a transparency group, a knockout group, a soft mask (§11.4, §11.6.5) | no | an offscreen layer per group per view |
| coverage as area (§8.5.3.3) and no disappearance for paths (§10.7.4) | no | MSAA counts samples |
| the same bytes on every adapter | no | rasterisation precision is the driver's |

## 7. What building it would mean

- **Where it would sit**: a lane for the *moving* view. During a gesture the hardware lane draws
  every step at a sampled coverage — sharp, never the blurry stand-in, and inside a 120 Hz refresh
  on every page this round measured — and when the gesture ends the compute lane draws the exact
  frame the oracle agrees with. That is the shape games use (a cheap frame in motion, the full one
  at rest) and the one ADR 2408's question asks for. Principle 5 binds the frame at rest; a frame
  in motion that is not the oracle's bytes is a departure the owner would take in writing.
- **What it needs in raster**: outlines resident once per distinct outline and instanced per
  placement (raster's outline store already holds them); clips in stencil bits; a fallback to the
  compute lane for a frame holding any mark the lane does not carry — a group with a non-default
  parameter, a soft mask, a blend mode, a zero-width stroke, a mark thin enough to vanish — or an
  offscreen layer per such group; and a gate of its own, since the corpus gate's bounds would hold
  it (section 5.1) and say nothing about §10.7.4.
- **The cheaper lever the heavy page names, independent of all this**: an image's samples resident
  on the device as a texture, sampled per view, instead of resampled and crossing on every step. It
  would move the plan sheet's step from 5.6–39.7 ms toward the probe's 1.3–2.4 ms without any change
  to path coverage; its price is the averaging of section 5.5, which it would have to reproduce in
  a shader to keep the oracle's bytes.

## 8. The question for the owner

**Q662**: is the hardware lane worth building as the moving view's lane, with the compute lane's
exact frame at rest — and is the resident image texture taken first, on its own?
`doc/questions/Q662` carries the recommendation and what the tree does meanwhile.
