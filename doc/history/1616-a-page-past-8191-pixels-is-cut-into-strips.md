# 1616 — A page past 8191 pixels is cut into strips

Slot 2 of batch ninety-seven, 2026-10-20, a pixels round on ADR 2058's lever. ADR 2068; ADR 2069
not used; no question. No ledger row moves: the planner decides where a page is divided.

**Premise held in substance, not in its count.** The witness at 1.25¹⁰ was one strip at any thread
count, refused by the range condition — but 21 020 marks of it, not four frame strokes
(`stroke_cut_census` labels them "zero-length", "curved" and "extent" by its own first match; with
the range alone changed every one of 7 895 rows is legal). **Hypothesis**: strip count held (16 of
16 asked), time held, "digests unchanged" did not — the strips differ from one strip at 39 235 to
48 017 of 88 M pixels, worst 2 levels, nearly all away from a cut: ADR 0219's shifted origin, which
every divided page carries, inside ADR 1758's bound. Looked at: single pixels on dots' edges.

**Built.** `render_cpu::strips::converted_row_by_row` asks `scan::MEASURED_LIMIT` (32 767) before
and after a strip's shift, not `SUPERSAMPLED_LIMIT` (8191); past it, ADR 0269's spikes keep their
rows. Three tests in `page_past_the_tile.rs`: a wide page cut and its strips the page to the byte,
a tall page cut with only a rectangle a level beside a cut, and trap 214's spikes (inside the
measured range cut, 10⁷ and 10²⁵ one strip). Calibrated: ADR 2058 undone in `fill_as` fails all
three; on HEAD the wide page and the inside spike are refused a cut.

**Measured** (one `--clock` hold, head/after exports, `window_against_whole` 1200 × 900): whole
page at 1.25¹⁰, 4 threads 20 716 and 23 736 → 6 560 and 4 508 ms, 24 threads 21 693 and 20 437 →
2 183 and 2 173 ms; its digest `a8b9c27e…` → `5f4d5366…` (4) and `9d4be8ec…` (24); the window's
digests HEAD's; 1.25⁸ every digest HEAD's.

**Found, not fixed** (ADR 2068 §5): a rectangle a cut crosses is a level from the page drawn in one
on any page — HEAD's 368 × 542 page, 144 pixels at 4 strips — because ADR 0139's exactness was
measured on the path converter and ADR 0476 sends rectangles to the rectangle converter.

**Gates.** `rustfmt --check`, `cargo fmt --check -p render-cpu` and `-D warnings clippy
--all-targets` 0; nextest `render-cpu` 212 passed; `cargo test -p conformance` 443 passed, 0
failed; `state.sh docs` 0. Six arms against `/home/AI/arms-1615/`: 0 digests and 0 mean errors
differ in each (968, 968, 968, 965, 965, 965 pages; the oracle states its strip count).
`raster_golden` held 974, moved 0, its division census line for line HEAD's (176 pages, 1 308
pixels, 3 levels). `hostile_geometry` 4 passed; `fixed_documents` 77 checked, 1 passed.

**Files.** `render-cpu/src/{strips,scan}.rs`, `render-cpu/tests/page_past_the_tile.rs`, ADR 2068,
`doc/state-of-play.md`, this.

**Unfinished.** The rectangle-at-a-cut level (`pdf_render`'s planner is where it closes).
