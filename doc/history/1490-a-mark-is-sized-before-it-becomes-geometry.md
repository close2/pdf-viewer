# 1490 — A mark is sized before it becomes geometry

Slot 1 of batch seventy-eight, 2026-10-10, a measure round: the owner's step back ("analyze if there
is a big picture change we are missing"). ADR 1817; no ADR 1818, no question. No ledger row moved:
§8.4.3.3 and §8.5.3.2 stay as they are, §10.7.4 stays `departed`; the row word yields to the target.

**Premise.** Held, with two corrections: the witness interprets in 2.27–2.55 s on HEAD (not 7 s; ADR
1813 cut it), and the device refuses after 2.23 s of host flattening at 96.0 B a mark.

**Hypothesis.** Held for its population, not for its lever. By ink extent at the window's fit
(1600 × 1000, 0.5938 px/unit) 4 365 065 of 4 408 405 marks (99.0 %) are under ¼ px, 2 927 922 inside
one pixel, and at four times the fit 4 365 065 are still under 1 px. Culling removes nothing at the
fit, and level of detail is forbidden by §10.7.4 (ADR 0368 (e)). What the marks cost is construction.

**Measured** (HEAD `bf2a9b00`'s export, `tile_price`, pinned, minimum of three, two clock holds):
fit page one thread 7 265.5 / 7 326.2 ms; without the 4 397 503 sub-pixel marks 44.6 ms — 1 656 ns a
mark against a 31 ns blend floor (138.2 ms). Callgrind at the fit: 157.9 G, drawing 112.4 G (25.5 k a
mark): exact-area fill 25.3 %, a pipeline built per mark 11.3 %, the stroker 13.4 %. Candidate A:
4× fit window 1 704.1 / 1 669.6 → culled 993.9 / 1 093.5 (0 px differ) → 2 × 2 tiles on four threads
376.0 / 419.5 ms; fit 2 × 2 3 102.5 / 3 382.1 ms (2.2×); seams 588 and 778 px. Device: each fit tile
fits and draws, 8 053 ms summed. `hayro-speed` at the fit: ours 9 237.8 ms, hayro 4 998.1 (1.85×).

**Decided** (ADR 1817): candidate B — classify a mark at the device transform before any outline,
edge or brush; a closed-form exact-area kernel for the census's population on the processor (priced
7.3 s → ~0.46 s, ≈ 16×), the same primitive on the device (the witness fits 256 MiB at ≤ 60 B a
mark), tiles from a culled list after, interpretation the next floor. `doc/todo/69` names them.
Evidence from pdf.js, okular/poppler, pdfcraft, mupdf (which widens thin strokes — a departure),
pdfium and Foxit is in ADR 1817 section 2, as approaches only.

**Unfinished.** Nothing built, by contract; candidate C, interpretation, named and not priced.
**Files.** `crates/render-raster/examples/tile_price.rs`, ADR 1817, `doc/todo/69` (all new);
`doc/todo/README.md` (one line); this record.

**Gates.** `rustfmt --check --edition 2024` on the example: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy -p render-raster --all-targets` in the HEAD export (the worktree's `render-cpu` held a
neighbour's mid-edit): exit 0. No test code touched, so no nextest. `cargo test -p conformance`: exit
0 (two runs 101 — my todo naming a session, this record at 41 lines — both fixed). `tile_price` as
two `--clock` walks: exit 0 (held 108.3 s after 616.8 s queued; 47.5 s after 122.3 s), with
`hayro-speed` in the first; callgrind as a small walk: exit 0, 430 s. Duration 3 300 s.
