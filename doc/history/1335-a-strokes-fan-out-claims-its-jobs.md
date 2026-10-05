# 1335 — A stroke fan-out's threads claim their jobs; a page's frames do not share the pool into cuts

Performance slot of batch fifty-two. ADR 1505. No ledger row moved; no question written. I did
not open `crates/render-cpu/`.

**Plans (ADR 1505 section 1).** The brief's four frames are eight: four photographs and four
`DCTDecode` masks at 2 480 × 2 630. A timeline probe, pinned: all eight start within 6.6 ms, masks
end at 17.7–29.6, the interpreter's own frame (the ninth thread on eight CPUs) at 42.9; busy
threads 66% of eight. The share rule (pool threads ÷ the page's large frames, counted from its
dictionaries; cut at a share of two) was built and lost unpinned: Plans 37.26 → 43.43 ms interp,
`arsimon_1.pdf` p56 30.75 → 34.37. Break-even on p56's frame alone: whole 15.8 ms; pass 4.6 plus
bands of 22.3 ms total work (1.41× the whole), 9.8 on eight threads — a cut pays only with most of
a pool idle. Reverted; `InFlight::Alone` stays.

**`bug1743245.pdf` (sections 2–4).** Callgrind: cutting 2 127 M (`split` 1 279 over 5.74 M
calls), separation 610 M over 3.87 M pairs (2.59 M settled by the earlier piece's edges), scan
939 M; 262 of 612 tilings not vouched whole, each on a real overlap. Five exact levers on the
tiling, all dropped (scan −370 M instructions but +0.6–1.6 ms; three others more instructions; a
buffer pool moved nothing). Kept: the fan-out's threads claim jobs (eight fixed shares finished
42–61 ms apart), shared expansions' makers first — claimed in job order the Type 3 page went 12.7 →
18.4. Pinned, interleaved against an export of the base: `bug1743245.pdf` 43.57 → 41.70 (step
41.02 → 39.88), `issue14415.pdf` 16.87 → 16.28, Type 3 13.00 → 12.43, text 7.82 → 7.53.

**Found, not built.** The photograph's turn is 76 ms (was 32): ADR 1495 declines a scan without
`EOI`, and its codestream has none (30.3 vs 70.4 ms interp with the decline removed). Type 3 and
`issue14415.pdf` read 13.1 and 17.3 on the commit before the base too — unbisected.
**Documents.** `doc/performance.md` rows re-taken 2026-10-04 (no 1334 rows arrived), earlier
takes kept; `doc/todo/36`'s encode and photograph rows; `46`, `47` already true.

**Gates.** `rustfmt --check` on `encode/parallel.rs` exit 0; clippy `-D warnings`: `raster-gpu`
and `render-raster` exit 0, `pdf-model` exit 101 on a sibling's untracked
`examples/display_list_census.rs` only. nextest: `raster-gpu` 668 passed, `pdf-model` 1 761,
`render-raster` 97, all exit 0. `cargo test -p conformance` exit 0, 373 passed. `jpeg_bands`
`-runs=0` on 216 seeds from `seed_streams.py` (pdf.js) exit 0. Behind the lock: `raster_golden`
held 974, moved 0; `pdf-model --test corpus` exit 0; `render-raster --test corpus` on base + this
change: 1× 966/0/1/7, 4× 962/0/4/8, 1-vs-N 0 at both, exit 0 — in the worktree 1× exit 101, the
device's refusal of `ContentStreamCycleType3insideType3.pdf` gone with siblings' changes;
`headless_gpu` 39 passed; `launch_path` exit 101, `xfa_filled_imm1344e.pdf` open_kinstructions
outside 1783–1820: worktree 1821.4, base 1819.4, base + this change 1823.4 — the 4.0 k is all in
libc reading `/proc/self/maps` (`getdelim`, `sscanf`, `strtoul`); no function of this tree moves.
