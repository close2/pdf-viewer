# 1491 — A stroke drawn a row at a time forbids no cut

Slot 2 of batch seventy-eight, 2026-10-10, a pixels round on the project owner's "improve
performance". ADR 1819; no ADR 1820, no question. No ledger row: the owner's target, not a clause.

**Premise, held.** `pdf_render::unsplittable_rows` forbade every stroke's whole extent: on the
witness at 150 dpi (4967 × 3509) 0 of 3509 rows, one strip. All 4 400 060 strokes have round caps —
3 704 964 axis-aligned and 667 897 oblique at or under a pixel (`stroke_cut_census`).

**Hypothesis, held in its result and not its reason.** No geometric rule of the brief's three frees
a row: a round cap is a curve at both ends of every stroke. The converter decides: `render_cpu::area`
evaluates each row from device coordinates, so a stroke it draws forbids no row, caps and slopes
included. `unsplittable_rows_where` asks the backend per stroke (`PlannedStroke`); `drawn_row_by_row`
says yes unless anti-aliasing is off, the composition replaces, the path curves, the width is under
one level, or the extent leaves `SUPERSAMPLED_LIMIT` under a strip's shift. `area::region` also
declined by height, so the frame was `tiny-skia`'s whole and `area`'s in a strip: `Region::runs`.

**The figure.** `quorra-transform render` of the witness, `--clock`, 4 rayon threads, interleaved:
**before 11.06, 11.06, 12.15 s wall; after 6.64, 7.74, 7.74 s**; user 10.8–11.0 → 13.0–16.8 s; peak
1.32–1.34 GiB both. Rasterising alone: 7.98 s whole, 2.94 s at sixteen asked (nine granted; 1983 of
3509 rows legal). No owner's-display figure: the contract's before/after has no window.

**Pixels.** One strip vs HEAD: 74 406 pixels up to 39 levels, all on marks past the cell budget —
the frames' edges, now exact area where `tiny-skia` supersampled (looked at: same shapes; an edge of
0.08 coverage HEAD dropped is drawn). Sixteen strips vs one: 13 892 pixels, 2 of them at 2 levels,
scattered over the hatching, 128 within two rows of a cut: the shifted origin, not the cut. Golden
held 974, moved 0. Division: 173 pages move (153 before), ≤ 946 pixels, ≤ 3 levels; three new past
the bound, painted and looked at, named in `PAST_THE_BOUND`: `bug1743245.pdf` 417/1, `issue14117.pdf`
608/2, `prefilled_f1040.pdf` 946/1. Six arms: 0 frame digests moved; the oracle's means moved on 5–7
pages at 1× (≤ 0.0013) and 12–16 at 4×, where big marks are now exact; same refused sets.
Calibrated (trap 13): the height decline put back fails `stroke_cuts.rs`'s frame (2 357 pixels),
`area` withdrawn its hatching by 16 levels. **Unfinished:** fills keep ADR 0139's rule, the next lever.

**Gates.** `rustfmt --check --edition 2024` on my seven Rust files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy -p pdf-render -p render-cpu -p pdf-model --all-targets`: exit 0. `cargo nextest run -p
pdf-render -p render-cpu`: 444 passed; `-p pdf-model`: 2036 passed, 19 skipped. `cargo test -p
conformance`: exit 0. `raster_golden --ignored` (`--tree 6`): exit 101, then with three pages named
exit 0. Six arms against `/home/AI/arms-1490/` (`--tree 12`, an export of HEAD + this patch, its own
target): each exit 0. `tools/state.sh turn` (`--clock`): exit 0, 33 banded, 0 outside. The A/B
(`--clock`): exit 0. Queued: arms 5 693 s, golden 1 950 s, turn 1 215 s. Duration 11 500 s.
