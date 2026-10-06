# 1555 — A pass's shading quads read one buffer of their numbers, each at its own offset

Status: accepted. Session 1360. Answers ADR 1541 section 4, which named a chain's region made ahead
of its first ask as the next lever on `bug1721218_reduced.pdf`'s zoom step and each render's pass
recording after it. Measured in wall time, the order is the other way round. Supersedes nothing.
Context: ISO 32000-2 §8.7.4.5, §8.5.4; ADRs 0011, 1471, 1491, 1529, 1541; trap 50;
`doc/habits/measuring.md` 63.
Code: `raster/crates/raster-gpu/src/compose/draw.rs` (new: `Shadings`, `Shadings::place`,
`Shadings::finish`, `Ready::Shaded`; `Executor::prepare_run`, `draw_pass`),
`device/rare.rs` (`Device::shaded_bind`, new: `Device::shading_stride`,
`Device::shading_buffer_limit`, `Device::shading_params_buffer`, `SHADING_PARAMS_BYTES`),
`pipeline/layouts.rs` (new: `dynamic_uniform_entry`; the shading layout's binding 0), `device.rs`.
Tests: `compose/draw.rs`'s `numbers_are_laid_at_the_stride_and_roll_over_at_the_limit` and
`a_paint_and_mask_seen_before_share_their_group` (each watched failing: the stride dropped, and the
mask left out of the key); the corpus gate's per-page digests on six arms.

**`crates/render-cpu/` was not opened.** Its figures were taken through `zoom_frame`'s
`ZOOM_FRAME_BACKEND=cpu` (ADR 1529).

## 1. What each lever is worth (callgrind by function, then the clock)

Callgrind, one thread, GPU lane, the 1.25× frame by subtraction of a one-frame run, exported HEAD:
`residue_intersection` 0.230 G, `draw_encoded` 0.240 G (two renders). Counted per region (habit
63): 3 515 regions a frame, every one a single link of 37 points (a dot) over a median of six
pixels, 42 000 instructions each, 72 edge deposits — inputs already as small as they come. In
`draw_pass`, per frame: 7 138 `create_buffer`, `write_buffer` and `create_bind_group` calls, one of
each per shading quad, 0.116 G between them.

The clock disagreed with the instruction counts, so each lever's ceiling was taken on the clock
before either was kept. Pinned, interleaved, quiet (load 2.2 to 2.4):

- **The regions.** A probe arm that skips every small region's fill (wrong bytes) took the step
  from 110.3–110.9 to 105.8–106.8 ms: the whole lever is about 4.5 ms. Built — every small chain
  made by helper threads ahead of the walk's first ask, admitted in encounter order as before,
  byte-identical by construction and watched so on a 256-cell fixture — it read 108.7–109.6: 1.6
  ms. These fills are short and cache-resident; their 0.23 G runs at an instruction rate the walk's
  other work does not. Set aside (section 3).
- **The pass.** Timed in a probe arm, `prepare_run` took 9.1 to 10.5 ms of each render's 3 583 ops,
  and each render's `draw_encoded` 22 to 32 ms. That is the larger lever by four times.

## 2. Kept: one buffer of numbers a pass, one bind group a paint and mask

What a shading quad reads is 176 bytes (`shading.wgsl`'s `Params`), a paint texture, the scratch
sheet and a mask. Every quad of a run is now laid into one buffer at the device's dynamic-offset
alignment; one bind group is made per paint, mask and buffer; and each draw names its offset.
`bug1721218_reduced.pdf` has five ramps a render and no mask, so a render makes one buffer and five
groups where it made 3 583 of each. A run whose numbers would pass the device's largest buffer, or
what a 32-bit offset addresses, closes that buffer and opens another, with groups of its own. The
image and function quads are unchanged: a page holds few of them.

**Byte-identical by construction**: each quad reads the same 176 bytes it read from a buffer of
its own, under the same pipelines; only where they sit has changed. The one layout that changed is
the shading quad's binding 0, now `has_dynamic_offset`, which nothing else binds.

## 3. Measured

Pinned to the eight fastest cores, minima of 5 rounds, four interleaved runs a tree, exported HEAD
and change trees in their own target directories (`md5sum`-distinct), load 3.2 to 3.5, the device
under 15% busy before each:

| `bug1721218_reduced.pdf`, GPU lane | HEAD | change | the CPU backend |
|---|---:|---:|---:|
| `zoom_frame`, the 1× frame | 116.0–128.5 ms | 89.2–95.5 ms | 47.0–47.2 ms |
| `zoom_frame`, the 1.25× step | 108.4–120.7 ms | 79.2–83.5 ms | 56.0–56.1 ms |

**The step is 1.41× the CPU backend in the same sitting**, minimum against minimum: inside 2×.
HEAD itself read 1.94× that sitting and 1.91× in another (110.2 against 57.7), not ADR 1541's
2.05×, on a quieter machine. The `doc/performance.md` §3e rows this moves are re-taken there.

**Corpus.** `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×, per-page
digests (`PDFVIEWER_RASTER_TIMES`) compared by name against exported HEAD: **0 pages moved on any
of the six arms**. Verdicts equal HEAD's; one-versus-many 0 on every run.

## 4. What is left

The regions made ahead are 1.6 ms of a 4.5 ms ceiling, and the made-ahead machinery (a plan of
small chains, a scope of helpers, a slot per chain) is about 300 lines: not taken for 1.5% of the
step, and recorded here so that it is not re-litigated without a new measurement. The pass's other
host costs (`CommandEncoder::finish`, the poll) and the walk's exact meets are what remains of the
step; the step's figure no longer asks for either.
