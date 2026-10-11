# 1500 — A fill drawn a row at a time forbids no cut, and a mark is suspected whole

Slot 5, batch seventy-nine, 2026-10-10, pixels; the owner's "improve performance". ADR 1837 only.

**Premise, held.** The planner asked the backend of strokes only (ADR 1819 §5); a fill's curves and
oblique edges forbade their rows whatever drew them. **Hypothesis, half held.** Fills forbid most
rows — 72 % alone, 92 985 of 128 609 at 150 dpi over 957 first pages, three quarters by curved fills
(`stroke_cut_census --corpus`) — but a curved fill reaches `area` as a glyph does and is freed, so
the gain is on text pages (`tracemonkey.pdf`, `issue840.pdf`), not only plans and forms.

**Built.** `PlannedMark` (`PlannedFill` beside `PlannedStroke`); `drawn_row_by_row` answers a fill
with the stroke's conditions less the curve and width ones. Two findings, both fixed: `area`
suspected a mark of overlap from its *surface's* rows, so a glyph cut by a strip was the integral
beside the cut and the set on the page (36 levels, `bug1820909.1.pdf`) — `suspected_beyond` asks
the rows clamped away, within the cell budget; and freed cuts replayed past 1.25 or balanced into
fewer strips (`shading_extend.pdf` 16 → 1, the witness 9 → 8) — each count is asked of the freed
rows and of those between fills, from one walk, keeping the most strips. **Oracle** (ADR 1837 §5):
`issue8702.pdf`'s 4× move is the gpu lane's — a frame edge at y = 3.824: exact 45, the lane's 63.

**The figure** (`--clock`, 16 threads, five interleaved, medians): `tracemonkey.pdf` at 4× **33.7 →
18.3 ms** (4 → 16 strips), at 150 dpi **17.8 → 11.7 ms**; `bug946506.pdf` 4× 71.6 → 34.2 ms;
`issue840.pdf` 33.6 → 29.6 ms. Rows forbidden over the corpus 128 609 → 35 720 (150 dpi); 42 pages
gain strips, none loses. The witness keeps HEAD's 9 strips at the same cuts (≈ 2.0–2.3 s both): its
rows are strokes' and clips'. No owner's-display figure: no window in the contract.

**Pixels.** Golden held 974, moved 0. Division: 175 pages move, ≤ 940 pixels, ≤ 3 levels;
`comments.pdf`, `highlights.pdf`, `issue7014.pdf` passed their ceilings by 2–3 pixels of one level
beside new cuts, painted and looked at, raised to 115/115/329. Six arms: 0 digests moved, means ≤
0.0008. Calibrated: `draw_fill`'s last draw to `tiny-skia` fails `stroke_cuts.rs`'s discs by 16
levels at the cut. **Unfinished:** strokes under one level (2169 of the witness's rows with clips)
await ADR 1829's closed form being asked; a fill too tall for one accumulator keeps its rows.

**Gates.** In an export of HEAD + this patch (own target): `rustfmt --check --edition 2024` on my
seven files exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-render -p render-cpu -p pdf-model
--all-targets` exit 0; `cargo nextest run` on the three, 2484 passed, 19 skipped. `cargo test -p
conformance` exit 101 only on `records`, naming 1497's and 1498's records at 41 lines; mine is 40.
`raster_golden --ignored` (`--tree 12`) exit 101 twice (six, then three pages), then exit 0 twice.
Six arms (`--tree 12`, before the both-rows plan, which moves no lane) each exit 0. `turn_path`
(`--clock`) exit 101 at 48 % device busy (2 outside of 21 judged), rerun exit 0, 0 outside of 33.
Census, A/Bs exit 0. Queued: census 1860 s, A/B 1373 s, turn 1393 s, golden 3653 s. Duration 12 900 s.
