# 1433 — A photograph's restart intervals are decoded beside each other, and its reduction is divided by rows

Status: accepted. Session 1299. Amends ADR 1271 section 5 (its "none of it reachable" is true of
`zune-jpeg`'s options, not of the codestream) and ADR 1321 section 5 (a turn onto a one-photograph
page now has a lever that needs no second image). Supersedes nothing.
Context: ISO 32000-2 §7.4.8, §7.3.8.2, §8.9.5.2, §11.6.5.2; ISO/IEC 10918-1 section F.1.2.3
(paraphrased); `CLAUDE.md` principle 2 and the rule on optimisations; `doc/questions/A121`;
`doc/habits/measuring.md` 45, 48, 50, 51, 52, 56; traps 50, 65, 71, 73.
Code: `crates/pdf-model/src/image/restart.rs` (new: `decode`, `layout`, `restart_intervals`,
`plan`, `band_codestream`, `decode_bands`), `image.rs` (`decode_jpeg` asks it first;
`eight_bit_device_tables`, `unpack_eight_bit_device`; `combine_on_the_finer_grid`'s per-column
sources and paired walk), `crates/pdf-render/src/paint.rs` (`Image::is_opaque`),
`raster/crates/raster-gpu/src/raster/reduce.rs` (`area_averaged` takes the device's threads),
`device/resident.rs`, `device/textures.rs` (`premultiplied`).
Tests: `restart.rs`'s four (seven `cjpeg` fixtures under `crates/pdf-model/tests/restart/`),
`image.rs`'s `the_table_route_unpacks_the_per_sample_routes_pixels` and
`a_combined_raster_is_its_sources_nearest_samples`, `reduce.rs`'s
`a_thread_count_changes_no_byte`, `raster-gpu/tests/reduction_threads.rs`.

## 1. What was measured, and what it said about the brief

`examples/frame_budget` (the page turn, 1 600 × 1 000, pinned to the eight CPUs whose
`cpuinfo_max_freq` is highest, minimum of five rounds, two runs an arm, each arm a binary of its
own target directory told apart by `md5sum`), callgrind on `examples/callgrind_interpret` with
`RAYON_NUM_THREADS=1`, and a scratch pair of `Instant`s around each step of raster's
`ensure_paint_textures`, removed since.

**The photograph is 5 280 × 3 792, not 2 700 × 3 450**, 4:2:0, with a `DRI` of 330 MCUs — one
MCU row an interval. Its `interp` (72 ms) is `zune-jpeg`'s: Huffman 49%, IDCT 13%, the four-lane
colour conversion 10%, upsampling 3%, and **one pass of this tree's that was not the codec at all**:
`element_alpha_is_shape` 11.3% — `pdf_render::Image::is_opaque` asking every one of twenty million
alphas one at a time, for a knockout group's report.

**Its `transfer` (53 ms) is not a transfer.** The bytes that cross are 20 MB (the 2 640 × 1 896
reduction the turn draws), and the queue takes them in **2.1 ms**, about 10 GB/s. The rest is the
host: **`area_averaged` 52.9 ms** on one thread, and `premultiplied`'s opacity test 1.1 ms. The
same page's zoom step uploads the whole 80 MB in 11 ms through `write_texture`. So the brief's
"0.7 GB/s" was the reduction divided by the bytes.

`images.pdf` has no codec cost to speak of: five `FlateDecode` `DeviceRGB` images under
`DeviceGray` soft masks, and callgrind put **`unpack` at 48%, `apply_soft_mask` 32% and
`raw_sample` 14%** — this tree's per-sample paths. `22060_A1_01_Plans.pdf`'s four photographs fan
out under ADR 1321's pool as that ADR says; what each one paid serially was the soft-mask
combination (44% of the page, two integer divisions a pixel) beside the `ICCBased` conversion
(32%, already banded).

## 2. The levers, each alone (pinned, milliseconds, the photograph's turn)

Each arm is the finished binary with the others switched off by a scratch environment variable,
so a row is one lever against the same tree:

| arm | turn | interp | transfer |
|---|---|---|---|
| the tree before this ADR | 126.7–127.5 | 72.2–72.7 | 53.1–53.4 |
| every lever off | 123.6–124.3 | 73.4–73.8 | 48.9–49.0 |
| restart bands alone | 90.2–93.1 | **35.9–39.9** | 51.8–52.9 |
| `is_opaque` by blocks alone | 121.4–121.8 | 70.8 | 49.2–49.6 |
| the reduction on the device's threads alone | 95.0–99.0 | 75.3–76.2 | **18.3–21.5** |
| all | **49.9–50.0** | 30.8–31.8 | 17.0–17.8 |

Unpinned — what a person gets, habit 48 — three runs at load 14–24: before 129.3–205.4 (interp
72.6–105.3); after **40.2–45.5** (interp 25.7–27.8, transfer 12.1–15.8).

- **Restart bands** (`image/restart.rs`). ISO/IEC 10918-1 section F.1.2.3 resets the DC
  predictions and the bit reader at each restart marker, so the entropy-coded data after one
  decodes with nothing before it. A frame whose intervals begin on MCU rows is cut into bands, each
  a codestream of its own (the header segments with `Y` rewritten, the scan header, its intervals,
  `EOI`), decoded on rayon's pool with the calling thread drawing bands from the same counter.
  Each band decodes one interval-aligned stretch above and one MCU row below what it keeps,
  because `zune-jpeg`'s fancy upsampling reads the chroma rows either side of an MCU boundary and
  replicates at a frame's edge; every kept line then has the neighbours it has in the whole frame.
  The last band is handed the scan's tail exactly as the data carries it, because the decoder pads
  an exhausted scan differently from one a marker ends. Anything the module does not admit —
  progressive, a second scan, a marker out of order, a count that does not add up, a sampling shape
  other than 1×1/2×1/1×2/2×2 over 1×1 — or any band the decoder refuses, falls back to the whole
  decode. Callgrind on one thread: 1 063.7 M → 1 215.2 M Ir (`decode_mcu_block` 525.8 → 587.6 M,
  the overlap); the wall clock is what moves. **Band height**, pinned: 128 lines 31.5–33.5 ms of
  interp, 256 30.8–31.8, 512 29.9–30.0, 1 024 30.2–30.9; unpinned 256 25.7–27.8, 512 26.8–36.6,
  1 024 31.6–42.0. `BAND_LINES` is 256, for the unpinned column. `BANDED_FLOOR` (1 M samples) keeps
  small frames whole.
- **`is_opaque` by blocks**: 64 alphas folded with `&`, the block tested once. 120.1 M → 21.6 M Ir,
  about 2.6 ms of the interpretation. Same answer by construction.
- **The reduction on the device's threads** (`reduce::area_averaged`, `Options::encode_threads`):
  the caller's `pdf_render` reduction already divides its rows under rayon; raster's mirror did
  not, on the argument that it runs once per `(image, factors)` — true, and that once is the page
  turn. Output rows are drawn from one counter sixteen at a time by a `std::thread::scope`;
  byte-identical because an output row reads only its own band of source rows. 52.9 → 16.1–17.7 ms
  on eight CPUs.
- **`premultiplied`'s opacity test by blocks**, the same fold: 1.1 → 0.55 ms on the turn, 4.6 →
  2.1 ms on the zoom step's 80 MB — the step's transfer 12.3 → 9.8.

**And for the other two witnesses**, the same binaries (pinned, load 18–20):

| page | row | before | after |
|---|---|---|---|
| `images.pdf` | turn (interp) | 156.0–157.9 (141.9–143.5) | **50.9–51.6 (42.7–42.9)** |
| | step (transfer) | 22.6 (18.8) | 12.4–12.7 (8.3–8.6) |
| `22060_A1_01_Plans.pdf` | turn (interp, transfer) | 128.0–131.6 (93.9–98.4, 24.8–25.1) | **93.4–93.9 (75.2–76.6, 8.1–10.2)** |
| | step (transfer) | 49.2–49.3 (41.7) | **23.0–23.5 (15.1–15.7)** |

- **Eight-bit grey and RGB unpacked by table** (`unpack_eight_bit_device`): where `sample_rgba`'s
  answer is `Decode::channel`'s entries and nothing else — eight bits, no matte, no colour key —
  a row is one pass pairing bytes with pixels. `images.pdf`: `unpack` + `raw_sample` 2 871.7 M →
  299.7 M Ir.
- **The soft-mask combination** (`combine_on_the_finer_grid`): a column's source is asked once per
  column rather than per pixel, and a pair on one grid walks three iterators in step.
  `apply_soft_mask` 1 488.9 M → 279.3 M Ir on `images.pdf`, 2 922.5 M → 548.1 M on the plan.
  Whole pages: `images.pdf` 4 631.2 M → 854.2 M, the plan 6 626.3 M → 4 257.6 M.

The text page (ISO 32000-2 p101) and the mesh page are unmoved inside their spread.

## 3. What was measured and not kept

**A staging buffer filled by the device's threads** in place of `write_texture`. `wgpu` creates a
buffer mapped at creation without `MAP_WRITE` as a second staging buffer, zero-filled, and copies it
on unmap: the zoom step's transfer went 9.8 → 14.9 ms. With `MAP_WRITE` the mapping is zero-filled
on this thread before a byte is written (`wgpu-core`'s `map_buffer`), and the step read 11.8/11.7
against the queue's 13.1/11.0 at load 11.7 — nothing. `write_texture` is one copy into staging
memory already, and there is no road here with fewer.

## 4. What was not built, and why

- **Decoding at the size the page needs.** The CPU oracle rasterises from the full image, and a
  raster drawn from a smaller decode would move pixels the cross-backend comparison is built on.
- **A three-channel texture.** WebGPU has no eight-bit three-component format; the four bytes are
  the smallest texel the device takes.
- **Uploading in bands as the decoder finishes them.** `A121` forbids presenting a page whose
  photograph has not arrived and a display list whose contents depend on whether a decode has
  finished; an upload that presents nothing and changes no command is inside its words. It is not
  built because it would cross three layers — the decode is `pdf-model`'s, on a thread that has no
  device, and the texture is raster's reduction of the finished `ImageSpec` — and what it could
  overlap is now a 16 ms reduction and a 2 ms copy.
- **The four-byte raster built in place.** The direct decode already writes the raster (ADR 1271);
  the banded path copies each band's kept lines once (libc copies 47.3 M → 67.6 M Ir), in parallel,
  because a band's decoder also writes the lines it discards.

## 5. The population, and what is left

One walk over 1 295 corpus files behind the lock (raw `DCTDecode` codestreams, not those behind a
second filter): 957 baseline frames, 357 of them with a `DRI`; of the 168 at a megasample or more,
76 carry one and **53 are admitted**, every one with an interval of exactly one MCU row. What is
left on the photograph's turn: the three serial walks over the entropy data (`DNL`, restart
intervals, `contradicted_frame`, about 9.4 M Ir each) before any band starts, the reduction's
per-block arithmetic, and a photograph with no `DRI`, whose Huffman pass stays one thread's.
