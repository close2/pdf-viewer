# 1583 — A step's `finish` and poll are its draws and its device, and are priced rather than taken

Status: accepted. Session 1373. Answers ADR 1555 section 4, which left `CommandEncoder::finish`
and the poll as the 1.25× step's remaining host costs on `bug1721218_reduced.pdf`, measured as a
total and never divided. Supersedes nothing.
Context: ISO 32000-2 §8.7.4.5, §11.4; ADRs 1471, 1529, 1541, 1555, 1582; `doc/habits/measuring.md`
68.
Code: none kept. Probed in an export of HEAD (scratch spans around `compose::submit_and_wait`'s
`finish`, `submit` and `poll` and around `readback::read_back`, removed since).

## 1. Measured by the clock (habit 68)

Pinned, minima and medians of 15 renders each (three runs of five rounds), load under 3. A step
draws the group as two frames, each read back (ADRs 1471, 1529), then the page's own frame:

| the 1.25× step, per render | `finish` | `submit` | poll | readback |
|---|---:|---:|---:|---:|
| the chromatic frame | 1.43 / 1.46 ms | 0.38 / 0.40 | 1.28 / 1.36 | 0.52 / 0.54 |
| the black frame | 1.42 / 1.45 | 0.38 / 0.40 | 1.25 / 1.32 | 0.49 / 0.64 |
| the page's frame | 0.06 / 0.06 | 0.06 / 0.07 | 0.48 / 0.85 | 0.49 / 0.72 |

About 8 ms of the step's 78, by minima: `finish` 2.9, `submit` 0.8, poll 3.0, readback 1.5. The 1×
frame's first render pays 4.3 ms of `finish` against 1.4 for every later one; that is the turn's
first sight, not the step's, and is left unattributed here.

## 2. `finish` is the passes' draws, and setting the pipeline once does not move it

wgpu-core 30 records a pass and encodes it into the driver's commands when the encoder finishes:
`encode_render_pass` is 64.1 M instructions over the zoom pair's four group renders and two page
frames (callgrind, one thread), about 16 M a group render, for 3 583 shading quads each drawn as
`set_pipeline`, `set_bind_group` at its dynamic offset (ADR 1555) and `draw` — about 4 500
instructions a quad, RADV's emission included. Each quad sets its pipeline, which wgpu re-binds
even where it is the one already bound, so a pass that sets a pipeline only when it changes was
built: `finish` read 1.43 / 1.46 ms over 20 renders, as before. **Not kept**: a change no clock sees is the
speculative optimisation `CLAUDE.md` forbids.

**The lever, priced.** A run of consecutive shading quads under one pipeline, paint and mask drawn
as one instanced draw, each instance reading its 176 bytes from a storage array at its instance
index rather than a uniform at a dynamic offset: up to about 1.4 ms of `finish` a group render, 2.8
a step, and some of the device's per-draw state in the poll. Byte-identical by construction — the
same numbers into the same shader arithmetic, and a draw's primitives blend in order — but it is a
shader binding, a layout and a grouping pass in `compose/draw.rs`, so it is a round of its own.

## 3. The poll is the device drawing, and the lever is overlap

The poll is the device executing what the render submitted: 1.25 to 1.36 ms for a group frame,
0.5 to 0.8 for the page's. The host waits because `frame_of` reads each group frame back before it
walks the next. Submitting the chromatic frame and walking the black one before reading it back
would overlap at most one frame's poll and readback, about 1.8 ms a step — and only on a page drawn
as two frames. It needs `Device::render` divided into a submit and a collect, and the composite to
hold a pending frame. **Priced, not built**: 2% of this page's step, against an interface change
on the device every host draws through.
