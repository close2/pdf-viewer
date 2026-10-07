# 1668 — The first frame's excess is the page's own work, and what a warm-up could make for it is not moved

Session 1416. Status: **accepted**; nothing is built but the instrument. Answers
`doc/QUORRA_FEEDBACK.md` section 9 (the per-device resources a first frame creates, made on the
warm-up thread instead). Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("[n]othing on the launch path waits for warmth"; "genuinely" is
decided by measurement); ADRs 0031, 0040, 1606, 1658; habit 59; traps 101 and 110.
Code: `crates/render-raster/examples/first_frame.rs` (each frame printed stage by stage from
`FrameCost`, its minor page faults, raster's named phases for frames 1, 2 and 10, and a marker
between frames that `strace` and `callgrind --dump-before` split at). The arms measured below were an
environment-selected patch to `raster-gpu` and `render-raster`; none of it is in the tree.

Every figure is ISO 32000-2 page 7 on the Radeon 890M through RADV, the CPU coverage lane, the
example built `--release` from this tree, run pinned to CPUs 0–3 and 12–15 with no display behind
the heavy-walk lock, arms interleaved run by run. "Steady" is the median of frames 6 to 10 of the
same run, "excess" frame 1 less steady; medians with the range, milliseconds.

## 1. Re-taken at section 9's three targets

Eight runs a target, load 2.6 to 3.0:

| target | frame 1 | steady | excess | scene | encode | upload | readback | rest of the device |
|---|---|---|---|---|---|---|---|---|
| 596 × 842 | 10.27 | 3.95 | **6.70** (5.84–7.49) | +0.45 | +4.07 | +0.32 | +0.39 | +1.47 |
| 1191 × 1684 | 17.06 | 9.87 | **7.74** (6.06–9.96) | +0.50 | +4.38 | +0.32 | +0.70 | +1.08 |
| 2382 × 3368 | 40.54 | 22.71 | **17.87** (8.40–19.52) | +0.52 | +5.83 | +0.35 | +2.41 | +1.35 |

Section 9 recorded 13.3, 14.3 and 18.1. At 4× a further 7.6 ms falls after the device has returned,
in `rasterize`'s host pass over the 32 MB read-back raster; frame 2 pays it again (9.55) and the
range is two-valued, so it is that path's memory and not a device's first use — a window frame
reads nothing back. A second before the first frame still moves nothing the right way: ten runs a
target, load 1.2 to 2.2, the 1× excess reads 6.62 after one second against 5.77 without, and the
part that grows is the first submission's recording, submit and wait beyond its pass (+2.11 against
+0.97), which is ADR 1658's device going idle after 50 ms rather than any shader.

## 2. What the first frame makes that the tenth does not

Counted three ways: `callgrind` call counts at wgpu's API and HAL boundary, frame 1 against frame 10,
with a five-second settle so that the warm-up had finished under the instrument; `strace -f` between
the markers, main thread; and the faults column, split by a probe at the stage boundaries.

- **Two device-lived textures and their views**: `create_texture` 3 against 1 — the glyph atlas
  (2048 × 4096 R8, 8 MiB, made at the first glyph's flush) and the 1 × 1 white stand-in beside the
  frame's own target — and two more staging buffers, the atlas's tiles and the stand-in's texel.
- **Command-stream memory**: two HAL command encoders and one `vkAllocateCommandBuffers` of
  sixteen, and 25 driver buffer objects, every one under recording or submission (`begin_encoding`,
  `draw`, `copy_buffer_to_texture`, `submit`). The kernel sees 43, 20, 3 and 0 `GEM_CREATE`s in
  frames 1, 2, 3 and 10. This is ADR 1606's pool.
- **The page's coverage**: 130.4 M instructions against 51.9 M, and 57 M of the 78.4 M difference
  is the CPU lane rasterising each glyph once into the atlas (`deposit_inside`, `fill_mask_settled`,
  the flattening, the topology), with 46 threads spawned for its two fan-outs on the machine's 24
  processors — a frame's own when it has new geometry, as a page turn does.
- **First touches of host memory**: 2 028 minor faults in frame 1 at 1× (2 712 at 4×, four runs
  each), 668 in frame 2, about one from frame 4 — 1 574 inside the encode, 310 in the scene, 70
  after the submission at 1×. The atlas's CPU sheet is 85 of them: prefaulting the whole sheet before the
  frame (0.25 to 0.67 ms) took frame 1 from 2 177 faults to 2 092 and its encode excess from 3.81
  to 3.82 ms.

## 3. Moving the two textures, measured

One binary, three arms, twelve runs an arm at 1× and at 4×:

| arm | found by frame 1 | upload, 1× / 4× | excess, 1× | device excess, 1× | warm set done |
|---|---|---|---|---|---|
| as the tree is | — | 0.28 / 0.38 | 6.12 (5.09–6.73) | 5.69 | 6.18 after the device |
| made on the warm-up thread first | 24 of 24 | 0.18 / 0.20 | 6.15 (5.28–6.43) | 5.72 | 6.63 |
| made on it after the warm set | 5 of 12 at 1×, 11 of 12 at 4× | 0.29 / 0.19 | 6.01 (5.24–6.82) | 5.24 | 6.18 |

The first frame's upload is 0.10 to 0.18 ms quicker when the textures are ready, and nothing the
frame reports as a whole moves outside its own spread. Made first on the warm-up thread they cost it
1.15 ms, being the device's first textures, and delay the warm set by 0.45 ms, on a launch path whose
first frame records 5 to 6 ms after the device. Made after it, they finish 6.36 ms after the device
and lose the race to the 1× frame's upload seven times in twelve. Made synchronously at construction,
the upper bound: 0.16 to 0.23 ms to make, upload 0.26 to 0.15 at 1×, excess 6.11 to 5.66 inside
ranges of 5.44–6.52 and 5.30–6.63. At 4× the arms' whole-frame excesses differ by up to 7 ms in
section 1's host pass, which they do not touch, and their device excesses agree.

## 4. Decision

Not built. A warm-up can make two things page one uses, and together they are worth 0.1 to 0.2 ms of
a 6 ms excess, inside the frame's spread. Made early, they also put an 8 MiB texture on every device,
including those that never draw a glyph, where today the first glyph makes it. Principle 2's rule
decides it, as it decided ADR 1606. The command-stream memory is ADR 1606's measurement, and nothing
here re-opens it. The rest is the page's own work and does not exist before the page does: every
glyph's coverage first seen (4 to 6 ms at every target), the scene's resources (0.5 ms), and the host
pages its buffers first touch. So the rule that nothing on the launch path waits for warmth costs
nothing here. There is nothing a warm-up thread can have ready that page one measurably needs.

## 5. What is left

Two costs on the first frame are not first-use and stay open as levers of their own. The first is
the 4× host pass over the read-back raster, which frames 1 and 2 pay and a window does not. The
second is encode's per-frame fan-out, whose 46 thread spawns a page turn pays again. The question
would reopen on a driver whose first texture costs what a frame notices.
`examples/first_frame` is the instrument: `strace -f -e trace=ioctl,statx` and
`valgrind --tool=callgrind --dump-before='*first_frame*mark*'` split at its markers.
