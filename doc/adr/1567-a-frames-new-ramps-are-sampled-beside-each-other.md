# 1567 — A frame's new ramps are sampled beside each other, and the regions are the larger lever

Status: accepted. Session 1366. Answers what `bug1721218_reduced.pdf`'s 1× frame costs beside its
zoom step, which ADR 1555 section 4 left measured but not divided. Supersedes nothing; amends ADR
1555 section 4 in one respect, the size of the regions' lever, below.
Context: ISO 32000-2 §8.7.4.5.3, §7.10.4, §8.5.4, §10.7.4; ADRs 0011, 0023, 1389, 1471, 1529,
1541, 1555; `doc/habits/measuring.md` 63 and 68; trap 50.
Code: `raster/crates/raster-gpu/src/device/ramp.rs` (new: `sample_segment`, `texel_byte`,
`sample_ramps`, `PARALLEL_FLOOR`; `sample_ramp`; `ramp_color_at` now the tests' statement),
`device/resident.rs` (`Device::ensure_paint_textures`).
Tests: `device/ramp/tests.rs`'s `the_table_is_the_per_texel_statement_byte_for_byte` (600 ramps,
coincident stops, half levels; watched failing with the rounding's `>=` made `>` and with the
cursor's `>=` made `>`), `a_texel_byte_rounds_as_round_does`,
`ramps_made_beside_each_other_are_made_in_order`; the corpus gate's per-page digests on six arms.

## 1. Where the 1× frame goes

Callgrind, the bench profile, `zoom_frame` at 1× alone, collection toggled on
`FrameSlot::render`: 1.603 G instructions at one thread, of which the two renders' encode 1.233 G.
At eight threads with each thread counted apart, the frame's own thread runs 1.09 G of it; the
largest pieces there are the residue meet (0.343 G), the chain's region (0.227 G), the commit
(0.232 G) and the two passes' `draw_encoded` (0.264 G), and inside the last, sampling the ramps:
0.150 G, a third of it `roundf`, which the baseline x86-64 target makes a library call.

On the clock, scratch timers in an export (removed since), pinned, quiet: the chromatic frame's
walk 37 to 46 ms, of which the regions 14 to 19 (their fills 9 to 11, flattening 4.4 to 5.1, chain
numbers 2.0 to 2.5) and the chain's edges 5 to 7; the black frame's walk 16 to 22, its chain
numbers 3.3 to 4.9; the paint textures 3.4 to 4.3 ms a frame of each colour, of which sampling
the ramps 5.4 to 6.4 ms of the two and making their textures 1.4 to 1.6. The step pays no paint
textures — its ramps were made by the frame before — which is why ADR 1555 never saw them.

## 2. The two levers, ordered by the clock

Two probe arms, each wrong bytes, interleaved four times, pinned, load 1.6 to 2.5:

| arm | the 1× frame |
|---|---:|
| as built | 88.4–92.4 ms |
| every residue region answered as a full tile (no region, no edges, no exact meet) | 58.8–60.3 ms |
| every ramp's table left unsampled | 83.1–86.9 ms |

**The regions are the larger lever, about 30 ms against 3 to 6** — larger than ADR 1555's 4.5 ms,
whose probe skipped the small regions' fills alone and left their flattening, their chain numbers
and the edges and exact meets they lead to. But what of it can be built has been: the regions made
ahead on helper threads are ADR 1555 section 1's 1.6 ms — read here as the frame's other threads
being busy with its exact meets already, which this round did not measure apart — and what is left on the walk's thread is each chain's own exact work
— 3 515 dots of 37 points, 42 000 instructions a fill, inputs already as small as they come
(habit 63). No construction cheaper than that work itself was found, and it is recorded here so
that the next round on this page starts from the 30 ms and not the 4.5.

## 3. Kept: a frame's new ramps sampled beside each other

The ramps' lever is whole and exact. `sample_ramp` asked `ramp_color_at` at each of a segment's
texels, scanning from the first stop, and rounded each component with `f32::round`. Now a
segment's texels are walked with a cursor that only moves forward — the stops are validated
ascending at upload and `lo + (hi − lo) · j / last` is monotone in `j` — through the same interval
arithmetic, and `texel_byte` rounds half away from zero by truncation and the fraction, exact for
every `x` below 2²³. That is a fifth of the sampling. And `ensure_paint_textures` gathers a frame's
new ramps and has `sample_ramps` make their tables on up to the frame's encode threads, eight or
more of them at a time, in a scope that ends with the call; the textures are then made in the
order the encode named them. **Byte-identical by construction**: the same function of the same
stops whichever thread runs it, and held so by the tests above.

## 4. Measured

Exports of HEAD and of HEAD with this change, their own target directories (`md5sum`-distinct),
`zoom_frame`, pinned, minima of 5 rounds, four interleaved runs, load 1.8 to 2.0:

| `bug1721218_reduced.pdf` | HEAD | change | the CPU backend |
|---|---:|---:|---:|
| the 1× frame, GPU lane | 85.2–88.8 ms | 81.5–83.9 ms | 47.5–48.5 ms |
| the 1.25× step | 78.8–79.9 ms | 77.7–82.0 ms | 56.2–57.8 ms |

**The 1× frame is 1.72× the CPU backend, from 1.79× in the same sitting**; the step is 1.38× and
does not move. `doc/performance.md` §3e's turn row reads 159.23 ms, its `scene` 86.62.

**Corpus.** `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×, per-page
digests compared by name against exported HEAD (2 843 s behind the lock): **0 pages moved on any
of the six arms**, verdicts equal, one-versus-many 0. `raster_golden` on the change's export:
held 974, moved 0.
