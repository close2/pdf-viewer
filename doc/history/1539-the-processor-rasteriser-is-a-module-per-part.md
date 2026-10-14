# 1539 — The processor rasteriser is a module per part, and the witness's memory is its list

Slot 6 of batch eighty-five, 2026-10-14, a pixels round on the owner's switch to code improvement.
ADRs 1914 and 1915; no question. No ledger row changes status; §10.7.4's row names the moved files.

**Premise.** Held: `lib.rs` was 7 544 lines; HEAD cuts the witness's 1 000 rows into ten strips
on 12 and on 24 threads; the memory figures are `doc/history/1532`'s. **Hypothesis.** The list is
79 % of the heap's peak and the frame under 1 %, as named; but the peak is the read's, at the
join, so a list dropping what the drawer finished with is not the cut — the inflated stream held
through the join is (274 MiB). For (c) the ask was not the lever; the 64-row floor was.

**Built.** (a) `render-cpu`: ten new modules and two under `stroke`, named by what they are, unit
tests beside what they test, `pub(crate)` only where a sibling reads it (ADR 1914). (b) A massif
census of `quorra-transform` on the witness (ADR 1915 §3). (c) A floor of 32 rows: 12 and 16
strips, the draw whole 326 → 293 ms and 322 → 303 ms (medians), but `raster_golden`'s division
test fails (a new page past ADR 1758's bound, three past their ceilings), so it was taken back.

**Proof that (a) moved nothing.** The split alone, exported onto HEAD: `raster_golden` held 974,
moved 0; division 176 pages, ≤ 1 308 px, 3 levels, ADR 1882's figures; `cpu-1x` 968 and `cpu-4x`
965 pages against `/home/AI/arms-1534/`: 0 digests, 0 mean errors differ. The tree handed over
differs from that export in comment lines only (checked file by file with comments stripped).

**Unfinished.** The census's cut is `pdf-model`'s read: named, not built. 49 ledger rows still
name `render-cpu/src/lib.rs` in `code` for what is now a module; the file exists, so the checker
passes. The 4× arms of (c) were not walked once the division test had decided it.

**Files.** `crates/render-cpu/src/` (ADR 1914's table, and `images.rs` → `reduced_images.rs`);
`render-raster/tests/singular_transform.rs` (a comment's path); `doc/conformance/ledger.toml`,
`doc/crate-map.md`, `doc/performance.md`, `doc/todo/11`, `23`, `40`, `69`; ADRs 1914, 1915.

**Gates.** `rustfmt --check`: 0. `clippy -D warnings --all-targets`: `render-cpu`, `render-raster`
exit 0. `nextest -p render-cpu` 204 passed; `render-raster` `singular_transform` 2 passed. `cargo
test -p conformance`: this round's rows and paths pass; the failures left are siblings' (`pdf-model`
image paths, `viewer-qt` tests). Split walk, one hold (queued 369 s, held 646 s), exit 0:
`t2-raster_golden` 2 passed, `t2-drawn_while_read` 2 passed, `cpu-1x`/`cpu-4x` exit 0. Builds, exit
0 (409 s, 84 s). Census, `--long`, exit 0 (55 s). Clock hold (queued 116 s, held 122 s, load
1.4–4.4) exit 0: the pooled draw, and `t2-turn_path` 33 of 33 figures judged, 0 outside. Floor-32
walk (held 323 s): `raster_golden` exit 101, the division test; `drawn_while_read` 2 passed; 1× arms
exit 0; stopped at 4×, exit 143. Duration 9 600 s.
