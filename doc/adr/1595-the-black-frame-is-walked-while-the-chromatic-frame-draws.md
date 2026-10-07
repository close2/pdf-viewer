# 1595 — The black frame is walked while the chromatic frame draws

Status: accepted. Session 1379. Answers ADR 1583 section 3, which priced the overlap at about
1.8 ms a step and left it unbuilt. Amends ADR 1471 in one respect: a group's two frames are
submitted before either is read back. Supersedes nothing.
Context: ISO 32000-2 §11.3.4, §11.4.5, §11.7.2; ADRs 1471, 1517, 1529, 1583, 1594;
`doc/habits/measuring.md` 68.
Code: `raster/crates/raster-gpu/src/device/pending.rs` (new: `PendingFrame`, `Device::submit`,
`Device::collect`, `Device::complete`, `take_pass_query`), `device/render.rs` (`submit_scene`,
`submit_encoded`), `device/record.rs` (`run_frame` submits and does not wait), `compose.rs`
(`submit`), `readback.rs` (`PendingCopy`, `copy_out`, `wait_for`), `timing.rs` (`read_pass` waits
on its frame's submission); `crates/render-raster/src/scene/own_space.rs` (`submit_frame`,
`collect_frame`).
Tests: `raster/crates/raster-gpu/tests/instanced_shadings.rs`'s
`a_frame_collected_after_the_next_was_submitted_is_the_frame_render_draws`.

## 1. What is built

`Device::render` is phases 1 to 3 (`submit_scene`) and phase 4 (`complete`) back to back, and
`Device::submit` / `Device::collect` are the same two held apart for a readback frame: the passes
and the target's copy are submitted, and the collect waits for that frame's own submission — by
its index, never "the most recent", which would wait for the frame behind it. A group compositing
in four components (ADR 1471) and a luminosity mask of a pair submit the chromatic frame, walk and
submit the black, then collect both. A compute chain's frame completes in `submit`, because its
stamps sit in the device's one set of compute queries; a frame drawn while another holds the
device's pass query reports its `execute` by the wall.

## 2. What the black walk needs from the chromatic render, and when it is ready

All of it is host state, whole when the chromatic `submit` returns: the clip outlines the chromatic
walk uploaded (`ClipOutline`, ADR 1529), handed back as its walk returns; the coverage tiles,
residue regions and meets raster kept (`KeptMeets`, ADRs 1517, 1529), filled during its encode; and
the resources its walk uploaded. Nothing of its pixels: those are read at the collect, by
`resolve_blending` or `paired_values`, which read both frames. Everything the black frame writes
reaches the device through the queue — uploads, atlas tiles, its own target — and the queue runs in
order, so the chromatic passes have read what they read before any of it lands.

## 3. Measured (pinned, interleaved exports, md5-distinct, five rounds a run, three runs)

Per render, scratch spans removed since: the chromatic frame's wait 1.3–2.2 ms → 0.37–0.44, the
black frame's 1.2–2.4 → under 0.01; submitting the black frame behind the chromatic one costs
1.0–1.15 ms of `submit` where it cost 0.44. On eight frames of one device, frames 7 and 8, against
ADR 1594 alone: 71.0–72.2 → 69.4–69.8 ms and 73.2–74.3 → 72.0–72.6.

**The cost, written down.** wgpu-core takes a HAL command encoder from its pool for every command
encoder and makes one when the pool is empty, and a new encoder allocates sixteen Vulkan command
buffers on first use, about 3 ms in RADV (ADR 1594 section 4). A frame submitted while another is in
flight needs an encoder the pool does not yet have, so a fresh device pays that two or three more
times over its first frames: `zoom_frame`'s fresh-device step reads 76.3–78.9 against ADR 1594's
74.9–75.5, and `frame_budget`'s step 84.4–85.4 against 83.0–84.4. A window's device lives across
documents, so it pays once, and on the zoom pair the overlap has paid it back by the sixth frame.
