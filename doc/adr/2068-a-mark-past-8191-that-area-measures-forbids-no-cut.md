# 2068 — A mark past 8191 that `area` measures forbids no cut

Session 1616, slot 2 of batch ninety-seven, a pixels round under the project owner's switch to code
improvements and bug fixes of 2026-10-14. Status: **accepted** and **built** in `render-cpu`.
Clause: none moves — the strip planner decides where a page is divided, not what it shows; §10.7.4's
row stays `departed` (ADRs 1060, 1360, 2058). Context: ADRs 0139, 0219, 0269, 0476, 1758, 1819,
1837, 2058; traps 13, 134, 146, 214. Code: `render_cpu::strips::converted_row_by_row` and the
comment of `drawn_row_by_row`, `scan::MEASURED_LIMIT` made visible to it. Tests:
`render-cpu/tests/page_past_the_tile.rs`, three new.

## 1. The premise, and what held of the hypothesis

**Premise (ADR 2058 §3 and §5): held in substance, not in its count.** The witness at 1.25¹⁰ of the
fit, 11 176 × 7 898 pixels, was drawn in one strip at any thread count. `stroke_cut_census` on HEAD
(its own target 11 177 × 7 895): 0 of 7 895 rows legal; the planner's refusals were 21 020 marks, which the census labels by its own
first matching class — 1 334 "a zero-length subpath", 17 "curved", 19 669 "extent past the
converter's range". Every one of them was the range: with the range condition alone changed, all
7 895 rows are legal and sixteen strips are made of sixteen asked. "Four frame strokes" were four
of twenty-one thousand.

**Hypothesis: the strip count held, the time held, "digests unchanged" did not.** The page is now
cut into 2, 4, 8 and 16 strips for those counts asked (replay 1.003 to 1.071). Its whole-page time
fell from about 21 s to 2.2 s at 24 threads (§3). But the page drawn in strips is not the page
drawn in one to the byte: 39 235, 45 466 and 48 017 of its 88 million pixels differ at 2, 4 and 16
strips, worst 2 levels, and only 6, 39 and 469 of them lie within two rows of a cut. That is ADR
0219's shifted origin — a mark in a strip below the first is mapped at another magnitude — which
every divided page already carries, inside ADR 1758's bound of one level per coverage and three
where several meet. The planner's rule cannot make a page's division free of it; ADR 0219 says why
no arrangement of this crate's arithmetic can.

## 2. Decision

**`converted_row_by_row` asks a mark's extent of `area`'s range, `scan::MEASURED_LIMIT` (32 767),
before and after any strip's shift, and no longer of the library's, `SUPERSAMPLED_LIMIT` (8191).**
The condition was written while a mark past 8191 lost its anti-aliasing before `area` was asked
and went to `tiny-skia`'s aliased converter, which clips at a strip's edge. Since ADR 2058 such a
mark reaches `area` on the page and in every strip alike — `fill_as` and `mask_fill` ask
`measurable`, the same bound — and `area` evaluates each row from the mark's own device
coordinates, which is ADR 1819's reason for freeing a cut. What still goes to the library is
kept: past 32 767, or with no extent, the mark's rows stay forbidden, which is ADR 0269's hostile
geometry (§4). Every other condition stands as it was.

**What the division costs is stated, not removed.** A whole page past 8191 now has the digest of
its plan, which is its pool's width, as every page under 8191 already had (trap 134); a gate that
holds such a page's pixels states its strip count.

## 3. Measured

One `--clock` hold (`scratchpad/r1616/hold.sh`, load 4.2–7.0 from a neighbour's arms just ended),
HEAD and HEAD plus this change, each its own export and target, `window_against_whole` on the
witness at 1200 × 900, interleaved twice per thread count:

| threads | 1.25⁸ (7 153 wide) | whole page at 1.25¹⁰ | window at 1.25¹⁰ | window against page |
|---|---|---|---|---|
| 4 | every digest HEAD's | 20 716, 23 736 → **6 560, 4 508 ms** | 196–265 → 239–298 ms, HEAD's digest | 6 085 → 5 478 differ, worst 2 |
| 24 | every digest HEAD's | 21 693, 20 437 → **2 183, 2 173 ms** | 180–200 → 164–166 ms, HEAD's digest | 6 075 → 5 425 differ, worst 2 |

The whole page's digest was `a8b9c27e671f0c0c` at both counts on HEAD and is `5f4d5366d6ead2d2` at
4 and `9d4be8ecbb6fef69` at 24 now; the window's digests are HEAD's. Under the census's own clock
at four threads the page was 15.9 s whole, 14.3 s in 2 strips, 6.5 s in 4 and 6.1 s in 16.

**Pixels outside the witness**: the six corpus arms of this change against HEAD's
(`/home/AI/arms-1615/`), by page: 0 of 968, 968, 968, 965, 965 and 965 frame digests differ, and 0
mean errors — the corpus gate draws its oracle at a stated strip count, so it sees no division.
`raster_golden`: held 974, moved 0, and its division census line for line HEAD's — 176 of 967
first pages move at some division of 2, 4, 8 and 16, at most 1 308 pixels and 3 levels, 12 named
past the bound: whatever marks past 8191 the corpus's first pages hold at 1×, freeing them
moved no page's division census.

**Looked at (trap 1).** The witness drawn in one strip and in sixteen, the densest tile of
differing pixels cropped (500 × 500 at (7000, 4000), 887 pixels): the same grid of round dots and
the same lettering in both, the differences single pixels on the dots' edges, scattered and at
most 2 levels — the shifted origin, not a seam.

## 4. What the old bound kept out (trap 214)

The range condition's `false` covered two populations: marks between 8191 and 32 767, which this
frees, and ADR 0269's spikes, which it must not. `page_past_the_tile.rs`'s
`what_the_old_bound_kept_out_is_cut_inside_the_measured_range_and_kept_past_it` draws ADR 0269's
368 × 542 page with a spike to (20 000.5, 9 000.25) — cut now, and its strips the page but for what
the page carries without it — and spikes to 10⁷ and 10²⁵, filled and stroked, whose plan stays one
strip. `hostile_geometry`: 4 passed; `fixed_documents`: 77 checked, 1 passed, `4605705.pdf` among
them. The two other new tests are a wide page (11 176 × 512, a frame and a grid of small marks
past and before 8191), its strips the page to the byte at 2, 4 and 16 asked, and a tall one
(640 × 8 704), cut through marks past 8191 rows. Calibrated: with `fill_as` planted to withdraw
anti-aliasing past 8191 again (ADR 2058 undone in one line), all three fail; on HEAD the wide page
and the spike inside the range are refused a cut.

## 5. Found, and not this ADR's to fix

**A rectangle a cut crosses is a level apart from the page drawn in one, on any page.** On an
ordinary 368 × 542 page with nothing near 8191, HEAD's four strips differ from its one strip at 144
pixels, each one level, each in the row beside a cut, each the edge row of a rectangle. ADR 0139's
table says an axis-aligned edge survives a cut exactly, and it measured `tiny-skia`'s *path*
converter; ADR 0476 later sent every rectangle to the library's *rectangle* converter, whose
clipped rectangle states the boundary row's product a level from the unclipped one's. A rectangle
forbids no row, so this has been inside ADR 1758's bound since ADR 0476 and no gate has named it.
On a page wider than 8191 it does not arise — every rectangle there is this backend's closed form
in every strip (ADR 2058) — and the tall-page test asserts exactly that residue and nothing more.
Closing it is a question for `pdf_render`'s planner (a rectangle's edge rows, or the closed form
at a strip's edge) and is left to a round that owns it.

## 6. What this does not do

The window path, `area`, `scan`'s converters and the device's lanes are unchanged: every mark is
handed the converter it was handed before, and what moves is where the page is divided, with ADR
0219's shifted origin for the marks in every strip but the first.
