# 1594 — A run of shading quads is one instanced draw, each instance reading its numbers by its index

Status: accepted. Session 1379. Answers ADR 1583 section 2, which priced the lever and left it
unbuilt, and attributes the first render's `finish` that ADR 1583 section 1 left unattributed.
Amends ADR 1555 in one respect: a quad's numbers sit in a window of its pass's buffer at its index,
not at an offset of their own. Supersedes nothing.
Context: ISO 32000-2 §8.7.4.5, §11.4; ADRs 0010, 0011, 1555, 1583; `doc/habits/measuring.md` 68.
Code: `raster/crates/raster-gpu/src/shaders/shading.wgsl` (`quads`, `VsOut::quad`),
`device/rare.rs` (`SHADING_QUADS_PER_WINDOW`, `SHADING_WINDOW_BYTES`,
`Device::shading_window_stride`), `pipeline/layouts.rs` (the shading binding's size),
`compose/draw.rs` (`Ready::Shaded`'s instances, `Ready::absorb`, `Shadings::place`).
Tests: `raster/crates/raster-gpu/tests/instanced_shadings.rs` (new), `compose/draw.rs`'s
`numbers_are_laid_by_index_in_windows_and_roll_over_at_the_limit`, `device/rare.rs`'s
`the_shading_window_is_the_shaders_array`.

## 1. What is built

The shading binding is a window of `array<Params, 93>`, a uniform the draw moves to at a dynamic
offset as before (ADR 1555), and each instance reads `quads[instance_index]` — the vertex stage at
`@builtin(instance_index)`, the fragment stage through a flat varying — into the one `params` every
function of the lane already reads. A run of consecutive shading quads under one pipeline, bind
group and window, of a style with one stage, is one `draw(0..4, first..end)`.

**Ninety-three** because 93 × 176 = 16 368 bytes is the largest window under 16 KiB, the smallest
`max_uniform_buffer_binding_size` any of wgpu's limit sets grants; a uniform array's length is fixed
where the WGSL is written. **A uniform and not a storage array** because a storage buffer read by a
vertex stage is a downlevel capability the GL backend this crate also loads does not always have,
and the window costs nothing a storage array would save: `bug1721218_reduced.pdf`'s group render
is 237 draws of every lane, where its shading quads alone were 3 583.

## 2. Why the pixels are the same, and the proof

A draw's primitives are rasterised and blended in their order, instance by instance, which is the
order consecutive draws have; each fragment reads the same 176 bytes it read at its own offset, and
the arithmetic after the read is the same text. A two-stage style — §11.4.6's knockout, erase then
deposit per element (ADR 0010) — is never merged, because its stages interleave per element.
`instanced_shadings.rs` draws 220 overlapping translucent shadings three ways — as runs, one per
draw, and each alone at the start of a window of its own — and asserts the bytes equal; it was
watched failing with the fragment stage reading the window's first quad. The corpus gate's digests
are the corpus-scale proof (the round's record).

## 3. Measured (pinned, interleaved exports, md5-distinct, five rounds a run, three runs)

`zoom_frame`, a fresh device a round: the 1× frame 79.0–80.4 → 76.3–77.9 ms, the 1.25× step
76.4–77.9 → 74.9–75.5. Eight frames on one device, frames 7 and 8: 71.8–73.2 → 71.0–72.2 and
75.0–75.9 → 73.2–74.3. A group render's `finish` after the first is 0.74–0.78 ms (scratch spans,
removed), where ADR 1583 read 1.42–1.46.

## 4. The first render's `finish` is the device's first command encoder

`zoom_frame` makes a device a round, so its 1× frame is a fresh device's first frame. Callgrind of
each `CommandEncoder::finish` over three rounds: the first group render's is 14.04–14.09 M
instructions, of which 8.63–8.68 M are RADV's `vkAllocateCommandBuffers`, and every later group
render's is 4.21–4.28 M with none. wgpu-core keeps a pool of HAL command encoders and makes one when
the pool is empty; a new encoder's first `begin_encoding` allocates sixteen Vulkan command buffers
at once (wgpu-hal's `ALLOCATION_GRANULARITY`). The cost is once per encoder a device ever needs at
once, so a window's long-lived device pays it on its first frame: about 3 ms of time-to-first-page.
**Priced, not built**: growing the pool on the warm-up thread would take it off the first frame,
and is a launch-path change for `launch_path`'s gate to judge.
