# 1505 — A thin segment is measured on the page's grid, and a curve that cannot fold forbids no cut

Slot 4, batch eighty, 2026-10-12, pixels; the owner's "improve performance", for which the
partial-row word yields (brief §3). ADR 1847 only; no ledger row moves.

**Premise, not held.** "2 169 of 3 509 rows still forbid a strip cut, by strokes under a level and by
clips": the census, given a tally of refusals by reason, says the backend refuses **61 strokes, all
curved** (round caps, wider than a pixel), and with those freed **0 of 2 169** rows stay forbidden;
clips forbid none. Same at the fit (0 of 618) and at 1× (0 of 1 037). **Hypothesis, half held**:
nine strips → sixteen held; "to under 1 s on four threads" did not — nine already fill four threads.

**Built.** `drawn_row_by_row` asks a curved stroke the draw's own `curves_may_fold` (`outlined_whole`)
instead of refusing every curve; a thin segment is admitted and measured on the page's grid in
`thin_segment::draw`, its rows moved to the surface by a whole number, and freed under a level too,
asked second (first, it cost 35 % more planning); a fill's span is asked at its own columns;
`PlannedStroke` gains `paint` and `blend`. Found on the way: since ADR 1829 a freed thin segment was
measured on the strip's grid — 64 221 pixels moved between the witness whole and in two strips; 6 467.

**The figure** (witness, 150 dpi, `--clock`, three interleaved, plan included): rows a cut may fall
on **1 340 → 3 509**; strips 9 → **16**; sixteen threads 1.10/1.19/1.93 → **0.95/1.53/0.98 s**;
four threads 1.42/1.77/1.42 → 1.41/1.42/1.42 s; planning 252–257 → 245–255 ms. No window here.

**Pixels** (looked at, trap 1): the page whole is unchanged — golden held 974, moved 0. The division
test named `issue9418.pdf` (467 pixels, 2 levels, glyph edges below the cuts its curved frame now
grants) and `prefilled_f1040.pdf` (1 308 against 946, one level, box edges at cuts): painted, ADR
1758's classes, named at those ceilings. Calibrated: the strip's grid back fails the dots by one
level; every curve refused fails the scenes' premise; pieces for every curve fail the rings by **16
levels**; the page's width back fails the tall fills. **Unfinished**: the zero-length dot as a disc
(`thin_segment` is both backends'); the strip count follows the processors, not the pool.

**Gates.** HEAD + this patch exported, own target. `rustfmt --check --edition 2024` on my files exit
0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-render -p render-cpu -p pdf-model --all-targets` exit
0; `cargo nextest run` on the three, 2511 passed, 19 skipped. `cargo test -p conformance` exit 101
only on `records`, naming 1503's, 1506's, 1508's in-progress records. `raster_golden --ignored`
(`--tree 12`) exit 101 on the two pages above, then (`--tree 6`) exit 0: held 974, moved 0; 176
pages move at some division, ≤ 1 308 pixels, ≤ 3 levels, 12 named. Six arms (`--tree 12`, one hold)
each exit 0, **0 frame digests moved**; the oracle's mean moved on 1–3 pages an arm, ≤ 0.0004.
`turn_path` (`--clock`) exit 101 without the sandbox worker, rerun exit 0, 0 outside of 30 judged.
Census, A/Bs, dumps exit 0. Queued: census 7 005 s, clocks 461–1 313 s, walk 5 378 s, golden 1 608
s. Duration 13 100 s.
