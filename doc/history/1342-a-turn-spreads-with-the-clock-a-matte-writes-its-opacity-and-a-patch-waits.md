# 1342 — A text turn spreads with the clock, a matte writes its opacity, and a decoder patch waits

Performance slot of batch fifty-three. ADRs 1519 and 1520; no ledger row moved; no question written
(Q227 gained a dated paragraph). I did not open `crates/render-cpu/`.

**The bimodal turn is the processor's clock.** ISO 32000-2 p101, pinned, exports with their own
target directories: only `encode` moves; one process's thirty rounds wander as much as twenty
processes do; address randomisation off, glibc's thresholds raised and the threads' cores logged
move nothing; the fan-out's work per glyph doubles. Interleaved and quiet: 7.6–9.9 ms as the gate
runs, 7.4–8.5 after 30 ms of spinning on the pinned cores, 8.8–10.8 after 1.5 s idle, 11.9–12.7 on
one core. The band stays spanning it; `turn-path.toml` says why.

**`issue13931.pdf`'s matte in one pass.** The inversion left alpha 255 and the multiplication wrote
`(255α + 127) ÷ 255`, which is `α` exactly, so the inversion now writes `α` where its plane is the
eager mask's own on the raster's grid and the multiplication is skipped: same bytes
(`a_matte_inverted_with_its_opacity_is_the_inversion_then_the_multiplication`, watched failing),
turn 23.55 → 21.73 ms at the minimum, interleaved; band re-taken 18.3 .. 26.9.

**The decoder's last row.** The brief's `hv_truncated.jpg` decodes correctly; the 128 row is ADR
1495's inline 100 × 107 frame. `doc/patches/zune-jpeg-scan-complete-without-eoi.patch` counts the
zeros appended past the data and flags `get_bits` taking more than the buffer holds, so a row stops
only once the decoder has consumed past the data. Scratch copy: the frame equals its `EOI`'d decode,
27 complete frames unchanged, no truncation of fifteen fixtures fewer correct rows, instructions
510.2 → 509.0 M; the tree under `[patch.crates-io]` passes but for the new guard, which fails as
written; `jpeg_bands` on it 55 216 runs in 240 s, clean.

**Gates.** `rustfmt --check` on my two Rust files exit 0. Clippy `-D warnings --all-targets`:
`pdf-render`, `render-raster`, `raster-gpu` exit 0; `pdf-model` exit 0 on an export of HEAD with my
files, 101 in the worktree on a sibling's `content/` mid-edit. nextest exit 0: `pdf-model` 1 772,
`pdf-render` 255, `render-raster` 100, `raster-gpu` 677. `cargo test -p conformance` 361 passed, 1
failed on two siblings' unfinished records. `jpeg_bands -runs=0` on 276 seeds exit 0, stock and
patched. Behind the lock: `raster_golden` held 974, moved 0, exit 0; `pdf-model --test corpus` exit
0; `render-raster --test corpus` 1× 967/0/0/7, 1-vs-N 0, exit 0; `headless_gpu` 39 passed;
`launch_path` 26 banded, 0 outside, exit 0; `turn_path` 8 of 10 runs exit 0 (two failed step rows
while neighbours built); re-banded, an export of HEAD with my files 20 of 20 inside, exit 0, and the
worktree 101 on the Type 3 row, 14.4 ms (export 10.6) — a sibling's mid-edit `raster/` meet.

**Left.** The patch waits on Q227, and then the cut plan's `EOI` decline. **Proposed trap.**
`turn_path`'s load ceiling counts processors, not the device: a neighbour's GPU work fails step
rows (2.5× on `issue19802.pdf`) at a load the gate admits.
