# 1726 — A knockout element's shape is the area its path covers

Session 1726, slot 7 of batch 111, a pixels round; 2026-10-23. ADR 2286; ADR 2287 and Q589 unused.

**Ranking.** Against HEAD's six arms the head is the glyph quantum's (ADR 0498): `copy_paste_ligatures`
1.1395, `issue16316` 1.0212, `endchar` 0.5882 on the `cpu` lane at 1x, pages cropped to a line of type,
0.0566, 0.0650, 0.0020 with the quantum off. Re-ranked at the quantum off (1x arms whole, 4x over
HEAD's sixty worst): the `gpu` lane at 4x on glyph pages (raster's ADR 0076, the ordered grid's five
levels on an axis-aligned edge), `issue2177` (ADRs 1435, 1443), then `knockout_groups_test` 0.3602 on
`cpu` and `compute` alike — the first page nothing names. Left as named: `issue18823`, `bug1802506`
(a border on its `/BBox` clip, `doc/todo/11`); `issue14165` is one level over an axial shading.

**Premise.** Not held: the worst three share no soft mask or knockout group. **Mark and clause.** Worst pixel (131, 126), 50 levels. The group's subsets isolate the red outline,
0.66 px wide, 0.13 of a pixel deep in that row: 222 alone on every backend, 191 inside the knockout
group on the oracle. §11.4.6's fractional shape, evaluated in `f64` from each outline's own area, is
raster's 84, 84, 168. A bare knockout element is Porter-Duff `Source`, which `scan::intersected`
declines, so its coverage was the library's lattice of sixteenths (ADR 2166 §3's open remedy).

**Fix.** `encode` draws every knockout `Fill` and `Stroke` with a coverage twin in ADR 1320's two
stages, clipped or not (`render-cpu/src/encode.rs`, one condition widened); images, nested groups and
twinless elements keep the bare draw. New test red at HEAD (363 channels off), green after.

**Pixels.** `raster_golden` 10 of 974 moved, raster only, re-taken (10 lines by targeted
replacement). Six arms of my export against HEAD's six arms: 0 frame digests moved (1x whole, 4x over
the 18 knockout pages and the movers); mean error down on every `cpu` and `compute` page-arm it moved
in (28); up on 9 `gpu` page-arms by at most 0.0651, the sampled lane's lattice the old oracle shared
(ADR 2286 §6). Drawn and looked at: `knockout_groups_test`, `alphatrans`.

**Files.** New: ADR 2286, this record, `render-cpu/tests/knockout_element_coverage.rs`. Edited:
`render-cpu/src/encode.rs`, `pdf-model/tests/raster_golden.tsv` (10 lines), §11.4.6's row (note,
test), `doc/state-of-play.md` (one sentence). For slot 4: no hunk in `scan.rs` or `area`.

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check -p render-cpu` 0; `-D warnings
clippy -p render-cpu --all-targets` 0; nextest `render-cpu` 247 of 247, `pdf-model --test
transparency_groups` 77 of 77; `cargo test -p conformance` 425 passed, 1 failed (`records` on round
1721's record, later fixed); `--test evidence --test adr_status --test records` 0. `tools/state.sh docs` withdrawn before its
grant, slot 4 mid-edit in `area` (habit 138): rustdoc `-D warnings -p render-cpu` in the export 0.
Export: `raster_golden` 964 held, 10 moved, re-taken, strips 0; every arm exit 0; `t2-turn_path`
under `--clock` at load 8.4 to 6.4: 21 judged, 20 inside, `images.pdf` step 12.53 ms against 12.20,
a page with no knockout group on a path `render-cpu` is not in. Duration 7 300 s, about 150 tool uses.
