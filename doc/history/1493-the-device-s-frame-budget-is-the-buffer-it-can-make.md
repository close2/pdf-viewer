# 1493 — The device's frame budget is the buffer it can make

Slot 4 of batch seventy-eight, 2026-10-10, a pixels round on the owner's "create the biggest PDF which
can be rendered on the GPU". ADR 1823; no ADR 1824, no question. No ledger row moved: a resource
budget is no clause's (§10.7.4 stays as it is), and the row word yields to the target.

**Premise** held: one constant, 256 MiB; the witness at 423 206 976 B on every lane. **Hypothesis**
(compute edges first) not held: the first site is the encoder's count before the walk, 96 B a command
(a rect and a quad instance), 4 408 406 × 96 exactly, refused after 2.1 s of scene with 0 ms of encode
— ADR 1817's "six flattened edges" is this count. With 4 GiB stated no other site fires on any lane.

**Built.** `heavy_drawing`: the witness's shape from a formula (A1, cubicle racks shaded by unit-step
`q … m l S Q` strokes, hatched rooms, dashed grid, Helvetica labels), chop the dial (505 920 strokes a
unit; chop 8 = 4 091 099 commands). Raster's default request is 2 GiB, held by `frame_ceiling` to
the adapter's `max_buffer_size` and storage binding (RADV 2 147 483 644 B, llvmpipe 128 MiB); the four
working-set shares stay at the 256 MiB they were measured at. Chunked submissions priced and declined
(the witness's largest stream is 282 MB of a 2 GiB buffer); "the adapter's memory" is not statable.

**The figure moved** (RADV headless, `Cpu` / `Compute`): largest size drawn chop 4 (2 067 419
commands) / chop 2 (1 055 579) → chop 12 (6 114 779) on both; the witness refused → drawn, 7 884 /
8 409 ms against the oracle's 7 590 ms. Chop 16 meets `max_resource_bytes` (512 MiB of one-segment
outlines), the next ceiling. Every size HEAD drew draws the same bytes. **Byte identity with the
oracle holds at no size** — ≤ 11 / ≤ 6 levels, inside the corpus bound, the same at chop 1 as at 8
(ADR 0082). Looked at: chop 8, and `issue12810.pdf` at 4×.

**Coordination.** `quorra` on RADV no longer refuses the witness (slot 3's refused-page path is not
taken for it); ADR 1817's device increment meets this count, not edges.

**Files.** `raster/crates/raster-gpu/src/{startup.rs, device/construct.rs, encode/residue.rs,
encode/expansion.rs, encode/parallel.rs, encode/instance.rs, encode/meet/kept.rs}`, `raster/doc/PLAN.md`,
`crates/render-raster/src/tiles.rs`, `crates/render-raster/tests/corpus.rs` (the 4× ratchet),
`crates/render-raster/examples/heavy_drawing.rs` (new), ADR 1823, this record.

**Gates.** `rustfmt --check --edition 2024`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` on `raster-gpu` and `render-raster`: exit 0. `cargo nextest run`: `raster-gpu` 707
passed, `render-raster` 100 passed. `cargo test -p conformance`: exit 0 (one run 101 on a neighbour's
record mid-write, then 0). Six arms against `/home/AI/arms-1490/`, own target
directory: no digest moved; five exit 0, cpu-4x 101 on the ratchet (`issue12810.pdf` drawn and
agreeing), edited and re-run: exit 0, 965 agree. `tools/state.sh turn` (`--clock`): exit 0, 33
banded, 0 outside. Ladders: five walks, exit 0. Duration 11 713 s.
