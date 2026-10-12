# 1511 — A page is cut into as many strips as its pool is wide, and the device is asked once

Slot 3 of batch eighty-one, 2026-10-12, a pixels round of the owner's performance target; the
partial-row word yields to it (brief §3). ADRs 1859, 1860; no question. No ledger row moves.

**Premise.** (a) Held in the code, not in the product: `plan_strips` read `available_parallelism`,
the global pool's width in every shipped configuration; only `RAYON_NUM_THREADS` (four in every
locked run) or an installed pool part them. The toolkit windows ask for whole pages: held. (b) Held:
seeds 1 788 / 667 ns, the device never asked again. **Hypothesis.** Held for the count and the seeds;
the toolkit cull is not a request extent alone — four hosts read a raster as the page, the stand-in
and the look assume it, and the tier is unknown at the first request (ADR 1859 section 4).

**Built.** The planner asks `rayon::current_num_threads`. `Costs` asks the device once on pages the
processor drew, a refusal its answer; seeds 253 and 120 ns, re-measured. `strip_count_price`.

**Figures** (witness; clocks at load 7–18, not judged, trap 149). Pinned, pool 4, pool-width vs
sixteen asked: 150 dpi 1 151 vs 1 238 ms; fit window 690 vs 761; 4× window 161 vs 185; 4× page 1 303
vs 1 409; on pool 8 its own width quickest in all five. Toolkit 4× zoom: page 1 303, band 444, window
161 ms. Xvfb, ten notches, last notch to landed frame, HEAD → this: `quorra-qt` 2.54, 3.47 → 3.41,
1.99 s; `quorra-gtk` 2.75, 7.62 → 2.50, 2.84 s; `quorra` 0.265, 0.237 → 0.171, 0.333 s, its first
draw 1 204, 1 059 → 718, 788 ms, one device refusal of 630 / 1 150 ms. Owner's display, `quorra`:
0.154, 0.148 → 0.142, 0.163 s; the device drew notch one in about 1.1 s, priced as measured after.

**Unfinished.** The toolkit band (ADR 1859 section 4; `viewer-host` and the toolkit hosts are
siblings' this batch). No photograph of four strips against sixteen: queued 6 062 s, stopped.

**Files.** `crates/render-cpu/src/lib.rs` (`plan_strips`, doc comments), `crates/viewer-ui/src/bin/
quorra/renderer.rs` (`Costs`, `Measured`, the refusal site, tests), `crates/pdf-model/examples/
strip_count_price.rs` (new), ADRs 1859, 1860, `doc/todo/69`, `doc/state-of-play.md`, this record.

**Gates.** In an export of HEAD plus this patch, own target. `rustfmt --check --edition 2024`: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy -p render-cpu -p pdf-model -p viewer-ui --all-targets`: exit 0.
`cargo nextest run` of the three: 2 415 passed; `viewer-ui` after the seeds 181. `cargo test -p
conformance`: 431 passed; at the end exit 101, `records` naming only 1509's, 1512's, 1514's. Clocks: sweep queued 1 611 s, held 162 s, exit 0; Xvfb gestures queued 217
s, held 213 s, exit 143 after their last run; zoom and owner's display queued 1 702 s, held 113 s,
exit 0; `turn_path` queued 310 s, held 114 s, exit 0, 33 banded, 0 judged, 0 outside. Walk (`--tree
12`, queued 1 704 s, held 6 114 s): `raster_golden` exit 0, held 974, moved 0, division 176 move, ≤ 1 308
pixels, ≤ 3 levels; six arms exit 0, 0 digests moved against `/home/AI/arms-1509`. Duration 13 400 s.
