# 1379: a run of shading quads is one instanced draw, and the black frame is walked beside the device

Raster slot, batch fifty-nine. ADRs 1594 and 1595; no ledger row moved, no question. The round was
cut by the machine's out-of-memory kill at 16:52 and resumed from the worktree at 22:11.

**The kill.** Named a suspect for the thread runaway: every added line under `raster/` and
`crates/render-raster/` was read, and none spawns, scopes or pools a thread; at the kill this
round's only process was an A/B waiting on the lock, having written nothing.
Every run after the resume went through `tools/bounded.sh --tree 12` behind the lock under
`ulimit -u 8192`; the exports from before the kill were deleted and made again. Threads of user AI:
218 before the first test run, 132 after; 295 at the most while the A/B ran.

**Built** (ADR 1594). The shading binding is a window of 93 quads' numbers, each instance reading
its own at `instance_index`; a run under one pipeline, group and window of a one-stage style is one
draw — 237 draws a group render. A test of three routes (runs, one per draw, each alone in its
window) was watched failing with the fragment reading the window's first quad. **Built** (1595). `Device::submit` and `Device::collect` are `render`'s phases 1–3 and 4 held apart, the wait by
submission index; a four-component group and a paired mask submit the chromatic frame, walk and
submit the black, then collect both. What the black walk reads of the chromatic one is host state
whole at its submit: clip outlines, kept meets and tiles, uploads — never its pixels.

**Measured**, pinned, interleaved exports, md5-distinct. `zoom_frame` fresh device, HEAD → ADR 1594
→ both: 1× 79.0–80.4 → 76.3–77.9 → 79.5–80.0; step 76.4–77.9 → 74.9–75.5 → 76.3–78.9. One device's
frames 7 and 8: 71.8–73.2 → 71.0–72.2 → 69.4–69.8, and 75.0–75.9 → 73.2–74.3 → 72.0–72.6.
`frame_budget`: turn 153.07–153.51 → 151.49–152.68, step unmoved. **Attributed**: the first render's
`finish` is the device's first wgpu command encoder, sixteen Vulkan command buffers allocated at
once, 8.6 M of its 14.1 M instructions (callgrind, every round); an overlapped frame pays another
while the pool grows, which is why a fresh device's step does not move. Priced, not built: growing
the pool on the warm-up thread.

**Docs.** `doc/performance.md` 3e's two rows (`taken` 2026-10-07); `turn-path.toml`'s bands by its
rule (turn 128.7–190.3, step 71.7–103.5); `doc/todo/36`, `45`, `state-of-play.md`. No feedback
section: the fresh-device ratios stay 1.68× and 1.38×.

**Gates.** `rustfmt --check` on the fourteen source files: 0. Clippy `-D warnings`, `raster-gpu` and
`render-raster`, all targets: 0. `cargo nextest run -p raster-gpu -p render-raster`: 0, 797 passed.
`cargo test -p conformance`: 0, twice. Behind the lock, exports of HEAD and of the change in their
own target directories: `render-raster --test corpus`, six arms each, all exit 0, digests by name 0
moved of 968 / 968 / 968 / 964 / 963 / 964, verdicts equal, 1-vs-N 0, in 1430 s and 1310 s;
`raster_golden` on the worktree: 0, held 974, moved 0, in 15 s; `tools/batch.sh raster-examples` 0,
14 passed, in 171 s; `turn_path` twice, each 0, 22 judged, 0 outside, in 82 and 82 s.
