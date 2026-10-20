# 1611 — A page past 8191 pixels is measured as its window is

Slot 5 of batch ninety-six, 2026-10-20, a clause round on ADR 1930 §4 (§10.7.4). ADR 2058; ADR 2059
not used; no question. The §10.7.4 row stays `departed`; its note gains a sentence and four tests.

**Premise held, hypothesis did not.** The difference was there (6 853 pixels, worst 58, at 1.25¹⁰),
but the window's corner is a whole number of pixels, the whole page has one digest at 1, 4 and 24
threads, and below 8191 pixels page and window agree to 2 levels (ADR 0219's shifted origin). The
cause was the page's size: 11 176 pixels wide, past `tiny-skia`'s 8191. The instrument's isolation
narrowed the worst pixel to one glyph's bar, 0 on the page and 58 in the window, 58 on a crop under
8191. Two mechanisms: `fill_rect` on a surface past 8191 is the supersampled converter (a
rectangle's closed form lost), and the range rule withdrew anti-aliasing from every mark reaching
past 8191 before `area` was asked (a quarter of this page aliased).

**Built.** `render_cpu::scan`: the range withdraws the library's anti-aliasing only (`fill_as`,
`fill_rectangles`, `mask_fill`), `area` measuring inside `MEASURED_LIMIT`; a rectangle on a surface
past `RECTANGLE_SURFACE_LIMIT` is written by its closed form into the coverage buffer
(`intersected`). `tests/page_past_the_tile.rs`, three tests failing on HEAD. The instrument sorts
differing pixels by edge distance and by the smallest mark's class, and isolates under
`WINDOW_AGAINST_WHOLE_ISOLATE`. **Measured** (one `--clock` hold, before/after exports, 4 and 24
threads): 1.25², 1.25⁵, 1.25⁸ every digest HEAD's; 1.25¹⁰ 6 853 → 6 085 differing, worst 58 → 2, the
window's digest HEAD's, the whole page 19 414 → 21 740 ms and 19 190 → 20 921 ms.

**Gates.** `rustfmt --check`, `cargo fmt --check` and `-D warnings clippy --all-targets` 0 for
`render-cpu` and `viewer-host`; nextest 209 and 278 (1 skipped); `cargo test -p conformance` 443
passed, 0 failed; `state.sh docs` 0. **The merge's `t3-fixed_docs` was red** on the first build
(`4605705.pdf` ink 98.824 against 145.204–147.204): `area` was handed ADR 0269's 10²⁵ spike. Fixed
by `MEASURED_LIMIT` (ADR 2058 §4); its pinning test red on the first build (108 px), green now;
`fixed_documents` 1 passed, `hostile_geometry` 4 passed. Six arms of the first build (no window is
in the corpus gate; their digests are the raster lanes', which no `render-cpu` change reaches, so
not re-run): 0 digests differ; mean error down on four pages at 4× only (`issue14497` 0.0509 →
0.0006, `issue19207` 0.0387 → 0.0120, two by under 0.001), both looked at. `raster_golden` held 974,
moved 0 (both builds). Drive's zoom steps in four windows: 12 works, 0 wrong.

**Files.** `render-cpu/src/{scan,area,strips}.rs`, `render-cpu/tests/page_past_the_tile.rs`,
`viewer-host/examples/window_against_whole.rs`, ADR 2058, the §10.7.4 row, `state-of-play.md`, this.

**Unfinished.** Past 8191 the page is one strip: its frame strokes pass the range and the planner
forbids their rows, now conservatively — a lever, unpriced. The 2-level residual is ADR 0219's.
