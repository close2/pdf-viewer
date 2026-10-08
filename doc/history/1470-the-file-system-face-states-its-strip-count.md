# 1470 — The file-system face states its strip count

Slot 3 of batch seventy-four, 2026-10-08, a pixels round. ADR 1774; ADR 1775 and Q366 not used. No
ledger row moved: the contract was a named build, not a clause.

**Premise.** Did not hold. `serve.rs` reads `available_parallelism` at line 174, but it draws with
`RASTERISING_THREADS.min(machine)`, and that is 1 on every machine. The face's renders never
followed the CPU count, and no held digest exists to regenerate: `read_corpus.rs` computes each
expectation live, at one strip. **Hypothesis.** Held, as stated.

**Built.** `serve::STRIPS = 1`, a constant of its own, stated on both routes by ADR 1742's rule:
the face's gate holds bytes and no clock. `confine` no longer asks the machine anything. The comment on
`RASTERISING_THREADS` had said `render-cpu` draws the same bytes on any core count, which ADR 0219
found false; it now says what is. Every other `CpuRasterizer::new()` outside a test was read (ADR 1774 section 4). Windows
and print keep the machine's count, for latency, since nothing compares their bytes.
`viewer-confined/src/worker.rs` repeats ADR 0139's withdrawn claim; that is named for its owner.
Taken from slot 6: `render-cpu/tests/interrupted_draw.rs`'s module comment now describes ADR 1780's
held-open draw.

**Gates.** Tier 1: `rustfmt --check --edition 2024` on the three Rust files, exit 0. `cargo clippy
-D warnings --all-targets` for `-p pdf-vfs` and `-p render-cpu`, exit 0. `cargo nextest run -p
pdf-vfs`: 79 passed, 2 skipped; `-p render-cpu`: 176 passed. `cargo test -p conformance`, exit 0.
Tier 2 ran from an export of HEAD plus this diff, with its own target directory. `read_corpus`:
exit 0 unpinned (198.3 s) and pinned to the eight fast CPUs (192.9 s), 3 217 renders their
generator's bytes, 0 not. `write_corpus`: exit 0 unpinned (34.7 s) and pinned (41.6 s). The plant
put the machine's count back and ran pinned: exit 101, 1 312 renders not the generator's. Six arms
against `/home/AI/arms-1468/`: 5 795 page lines, 0 digests and 0 means moved, each arm exit 0.
`raster_golden`, both tests: exit 0, held 974, moved 0. `turn_path` was **not run**. It is a
`--clock` gate, and slot 4's `--long` campaign held the second lane from 23:21 for longer than this
round's budget. `cargo tree` puts no `pdf-vfs` under `render-raster`, so its build graph is HEAD's.

**Unfinished.** `turn_path` twice, for the reason above. Widening the vfs worker's pool, which ADR
1554's arena limit now permits, is a speed question; `STRIPS` stays one whatever the width.

Duration 5 800 s.
