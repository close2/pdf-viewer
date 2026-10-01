# 1457 — An opaque band is reduced off its column sums, a matte is tabulated, the scan is walked once, and a frame without restart intervals is not cut

Status: accepted. Session 1311. Answers ADR 1433 section 5's "what is left" item by item; keeps
ADR 1433's restart bands, its row division of the reduction and ADR 1321's decodes ahead.
Supersedes nothing.
Context: ISO 32000-2 §7.4.8, §11.6.5.2, §10.7.4; ISO/IEC 10918-1 (held as ITU-T T.81, 09/92,
`doc/md/T.81.md`; cited by section and paraphrased) sections E.2.4, F.1.2.1, F.1.2.3, F.2.1.3.1,
F.2.2; `CLAUDE.md` principle 2 and the rule on optimisations; `doc/questions/A121`;
`doc/QUORRA_FEEDBACK.md` section 52; `doc/habits/measuring.md` 48, 52, 56; traps 50, 65, 73.
Code: `crates/pdf-render/src/paint.rs` (`Image::reduce_row`, `opaque_band`, `Reciprocals`,
`SMALL_BLOCK`), `raster/crates/raster-gpu/src/raster/reduce.rs` (the same, mirrored),
`crates/pdf-model/src/image.rs` (`matted_eight_bit_device_tables`,
`unpack_matted_eight_bit_device`, `MatteTable`, `Prematte::restore_component`, `FirstScan`,
`first_scan`; `defined_number_of_lines` reads the walk), `image/restart.rs` (`layout` and
`restart_intervals` read the walk), `crates/pdf-render/examples/area_bench.rs` (the turn's
photograph as a row).
Tests: `paint.rs`'s and `reduce.rs`'s `a_row_of_column_sums_is_the_per_block_arithmetic`,
`reduce.rs`'s `a_reciprocal_is_the_quotient`, `image.rs`'s
`the_matte_table_route_unpacks_the_per_sample_routes_pixels`.

## 1. What was measured first

`examples/frame_budget`, 1 600 × 1 000, pinned to the eight CPUs whose `cpuinfo_max_freq` is
highest, minimum of five rounds, three runs an arm, each arm a binary told apart by `md5sum`;
lever-alone arms from one binary with scratch switches, removed since. Callgrind on
`examples/callgrind_interpret`, `RAYON_NUM_THREADS=1`.

**The brief's premise, checked.** The census (1 477 corpus PDFs, raw `DCTDecode` codestreams):
966 baseline frames, 359 with a `DRI`; 168 of a megasample or more, **92 of them with no `DRI`**.
The three largest no-`DRI` pages are `arsimon_1.pdf` p56 (two 4 961 × 3 508 frames), `issue13931.pdf`
p1 (2 996 × 4 256) and `carp2010-2.pdf` p5 (4 651 × 1 543). **Their cost was not mostly Huffman**:
`arsimon`'s two frames are scanned white paper, a quarter `idct_int_1x1`; `carp` is Huffman 24%,
colour 15%, IDCT 12%. And `issue13931` is 4 823 M instructions of which **the codec is under 6%**:
its soft mask states a `/Matte`, and `unpack` (65%), `raw_sample` (10%) and the repacking in
`decode_dct` (14%) were §11.6.5.2's inversion asked per sample, two allocations a pixel.
**"Three serial walks" were two**: `defined_number_of_lines` and `restart_intervals`, 9.4 M
instructions each on the 5 280 × 3 792 photograph; `contradicted_frame` compares two pairs of
integers and walks nothing.

## 2. The levers, and what each did alone (pinned, milliseconds)

| page | row | before | after |
|---|---|---|---|
| `issue12841_reduced.pdf` (5 280 × 3 792, `DRI`) | turn (transfer) | 46.6–48.3 (16.4–17.0) | **42.3–43.7 (12.2–12.8)** |
| `issue13931.pdf` (`/Matte`) | turn (interp) | 178.8–181.2 (170.8–173.6) | **64.3–66.6 (57.6–60.3)** |
| | step (transfer) | 11.9–12.2 (10.3–11.0) | 9.6–10.3 (8.2–9.1) |
| `arsimon_1.pdf` p56 | turn (transfer) | 39.1–39.8 (8.8–10.0) | **37.1–37.8 (6.0–6.7)** |
| | step (transfer) | 10.0–10.8 (8.4–8.8) | 7.5–8.0 (5.8–6.4) |
| `carp2010-2.pdf` p5 | turn (transfer) | 22.4–23.0 (3.1–3.4) | 21.3–22.2 (2.2–2.7) |
| `images.pdf`, `22060_A1_01_Plans.pdf` | turn, step | 49.7–51.2, 85.7–88.1 | unmoved inside the runs' spread |

Load 3.2–4.0. Unpinned, two runs: the photograph's turn 37.7–38.0 → 35.4–36.3; `issue13931`'s
177.8–178.6 → 67.6–68.6.

- **The reduction reads an opaque band off its column sums.** `area_averaged` paid, per output
  cell, two row slices, four multiplications a sample and four 64-bit divisions: 54 ms on one
  thread for the turn's twofold reduction, 16–17 on eight. Now a band of source rows whose alphas
  fold to 255 (eight bytes at a time) is summed down each column once — a loop the compiler
  vectorises — and each cell is read off those sums with its mean taken by a reciprocal. Two
  facts make that the same byte, and both are proved in `paint.rs`: an opaque block's
  premultiplied `round_div(255·s, 255·c)` is `⌊(s + ⌊c/2⌋) ÷ c⌋`, and `⌈2³²/c⌉` times a numerator
  under `256·c` is that quotient for every `c` up to 4 095 (`a_reciprocal_is_the_quotient` checks
  each floor's edge). A band with one non-opaque sample takes the per-block arithmetic as
  before: asking per cell, without the band's fold, paid the sums *and* the old arithmetic and
  measured a third slower on the soft-masked pages (`Plans` transfer 9.5 → 13.5, its step 15 →
  18), which is why the fold is there. `pdf_render`'s oracle and raster's mirror change together,
  statement for statement.
- **A matte is tabulated.** §11.6.5.2's inversion, at eight bits in `DeviceGray` or `DeviceRGB`, is
  a function of the sample, `α` and the component and of nothing else, so 65 536 evaluations of
  the same calls per component — `Decode::value`, `Prematte::restore_component`, `channel` —
  answer every pixel; the JPEG frame's four-byte pixels are read in place rather than repacked,
  and rows divide across the pool. 4 823 M → 1 132 M instructions; 172 → 58 ms of interpretation.
  Floor 65 536 pixels; below it the per-sample route does no more work.
- **The first scan is walked once.** `FirstScan` records the scan's stuffed bytes and restart
  markers and the marker that ends it; the `DNL` reading and the restart intervals both read it.
  1 116.6 M → 1 107.9 M instructions on the photograph's page, under a millisecond — inside the
  wall clock's spread, and kept for being one walk where there were two rather than for the time.

## 3. Built, measured, and not kept: cutting a frame with no restart interval

`zune-jpeg` exposes no coefficients, so the pipeline the brief named is not reachable through it.
What was built instead, and not merged: one
Huffman pass recording at each MCU row the bit it starts on and each component's first DC
difference with the value it reaches (section F.2.1.3.1's prediction), then each band a codestream
of its own — the row's bits realigned to a byte, each first DC difference re-coded from a
prediction of zero by section F.1.2.1's categories, padded with one-bits and ended by `EOI`
(section F.1.2.3) — handed to the pool as soon as the pass has read past it. It was byte-identical
on six `cjpeg` fixtures (every sampling shape, an `-optimize` table) and on **all 125 corpus frames
it and ADR 1433 admit** (146 of a megasample or more that the decoder reads as `YCbCr` or grey).
It did not pay. The pass is 80% of `zune-jpeg`'s own Huffman work (`carp`: 51.6 M against 63 M
instructions, the page 253 M → 338 M), so a single photograph gained 2–3 ms (`carp` 14.4 → 11.3–12.8,
`issue13931` 58.5 → 55–57), and a page whose other images ADR 1321 already decodes on the pool lost:
`arsimon` 27.6–28.0 → 29.2–30.0, `Plans` 69–71 → 77–80 ungated and 70–73 with the pass refused on
pool threads. A gain that changes sign with the page's other images, measured in milliseconds
either way, is not a lever this tree keeps. What it would need: a signal from the content walk that
the page offered no decode ahead — the walk knows, and `decode_jpeg` is five calls below it.

## 4. Quorra's section 52, ask 2: priced, and re-asked with the bound

A reduction on the device needs the full-size samples resident first, and on this adapter the
upload of those alone is already measured — it is the step row: **80 MB in 10.3–11.0 ms**. The host
route after section 2 is 12.2–12.8 ms for the reduction and its 20 MB upload together. So the
most a device reduction could take off a page turn is about 2 ms, less whatever its compute pass
costs, and on the zoom step it takes nothing, since a step to twice the magnification draws this
photograph at its own resolution and uploads every sample either way. `A121` lets an upload start
early but not a page be presented before its photograph has arrived, and the samples exist only
once the decode has finished, so there is nothing earlier to start it on. No compute pass was prototyped: a bound of 2 ms is what a prototype would
have to beat, and the ask is re-put to quorra with it rather than closed.

## 5. What is left

The photograph's turn is now interpretation (28–30 ms, `zune-jpeg`'s Huffman 53% of it, already
banded) and 12 ms of transfer, of which the reduction's band sums and the 20 MB copy are most;
`issue13931`'s 58 ms is the soft mask's combination (`apply_soft_mask`, 268 M) and the mask's own
unpacking (166 M) beside the tabulated inversion (396 M). The no-`DRI` frames keep one thread's
Huffman pass.
