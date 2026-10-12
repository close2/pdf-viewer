# 1524 — A line's closed form is made ahead of the walk, and the device keeps its memory hint

Slot 3 of batch eighty-three, 2026-10-12, a pixels round of the owner's performance target; the
partial-row word yields to it (brief §3). ADRs 1884, 1885; no question. No ledger row moves.

**Premise.** Held, with one slip: no `MemoryHints` in raster's sources, but `request_device` is
`device/construct.rs:50`, not `startup.rs:407`. Callgrind at HEAD: 735 instructions a command on the
witness, about 450 of a line's pure. **(a)** held: a step's encode halved — after a first build
(forms a window at a time, clip resolved first) moved the walk only 274–342 → 193–221 ms. **(b)** half
held: bring-up faster and no digest moved, but every step's transfer paid, so it is reverted.

**Built.** `encode/lines_ahead.rs`: a ring of 4 096-position blocks three threads fill and the walk
empties, both walks through it, no thread below 16 384 lines; `Unclipped` shared by walk and threads;
a line's instance written where the walk meets it, a queued run noted as one batch.

**Figures** (clock hold, HEAD / +(a) / +(a)+(b) exports, witness at 0.594, 1.25×, 1.5×, 2×, fit, load
1.3–1.6). Step encode 1.25× `Cpu` 224.6, 231.3 → 127.3, 130.0 ms; `Compute` 227.0, 227.8 → 118.3,
121.5 ms; the walk 186–192 → 82–96 ms; the step 381–389 → 291–296 ms. Walk-thread instructions 3.2 G
→ 1.23 G an encode. (b): `request_device` 4.13–5.84 → 2.11–2.91 ms; `bring_up_ms` 18.04 → 15.99,
17.09 ms at 362–364 MiB, all in band; `issue12841_reduced.pdf`'s step transfer 1.20–1.25 → 2.72–2.75
ms, its step out of band in two of three runs. No Xvfb or owner's-display figure: `llvmpipe` refuses
the witness (ADR 1860), and the owner's loop was not this slot's.

**Bytes.** Witness digests, five views, both device lanes: HEAD's. Six arms: 0 pages moved.

**Unfinished.** (c), ADR 1877 §5's resident instances: past the budget. `MemoryHints::Manual` is
priced as the next lever (ADR 1885 §3), not built.

**Files.** `raster/crates/raster-gpu/src/encode.rs`, `encode/{lines_ahead (new),line,instance,
parallel}.rs`, `encode/parallel/commit.rs`, `tests/encode_threads.rs`; ADRs 1884, 1885,
`doc/todo/69` §5, `doc/state-of-play.md`.

**Gates.** `rustfmt --check --edition 2024`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p raster-gpu
-p render-raster --all-targets`: exit 0; `cargo doc -p raster-gpu` at `-D warnings`: exit 0. `cargo
nextest run -p raster-gpu -p render-raster`: 824 passed, 5 skipped, `line_lane` and
`function_conformance` among them; the two new tests fail with each of two defects planted. `cargo
test -p conformance`: exit 0. Six arms (`--tree 12`, held 11 412 s, about 9 800 s of it the machine
suspended): exit 0 each. Clock hold one (322 s): `turn_path` HEAD 0 outside, +(a)+(b) 1;
`launch_path` HEAD 2 (`cold_open_ms`), +(a)+(b) 0. Hold two (593 s), for the figures the first cut
off: `turn_path` HEAD 0, +(a) 0 twice, +(a)+(b) 0 and 1. Duration 19 930 s, about 9 800 s suspended.
