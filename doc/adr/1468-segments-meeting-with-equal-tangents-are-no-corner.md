# 1468 — Segments meeting with equal tangents are no corner, and a coarse tiling cannot fill a fine set

Status: accepted. Session 1316. Amends ADR 1455 section 2's last sentence ("A turn between two curves
that meet with equal tangents is still joined by the stroke's style between their chords"): such a
point is now joined by the set's disc. Answers ADR 1443's lever for `issue14415.pdf` by derivation.
Supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §8.4.3.4, §8.4.3.5, §10.7.2; ADRs 1375, 1397, 1443, 1455; trap 85;
`doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/stroke/centre.rs` (`Centre::tangent_continuous`,
`DIRECTION_ULPS`), `raster/crates/raster-gpu/src/raster/stroke.rs` (`stroke_subpath`'s joins).
Tests: `raster/crates/raster-gpu/src/raster/tests/equal_tangents.rs`.

**`crates/render-cpu/` was not opened.** Every expected value is the set's or the clause's.

## 1. The clause, and what was drawn

§8.4.3.4: "Join styles shall be significant only at points where consecutive segments of a path
connect at an angle". Two cubics whose tangents agree at their common point — the four of a circle,
an S-curve's inflection — or a line leaving a curve along its tangent connect at no angle. ADR 1397
already squares the pieces there to the chords (the tangents turn less than the chords do), so the
join was drawn between the chords in the stroke's style: a miter stands `hw · (sec(θ/2) − 1)` past
§8.4.3.2's set, a bevel leaves the disc's segment out, where `θ` is the flattening's own turn. The
brief's worry — a miter spike as the angle closes — is the other limit: §8.4.3.5's ratio
`1 / sin(φ/2)` grows as the angle *between* the segments goes to zero (a cusp), and at equal
tangents it is 1. Measured before the change, the three styles against round on a circle of four
cubics, `fill_mask` bytes: radius 30, half-width 12 — 4 pixels apart by 1 level each way; radius 8,
half-width 12 — miter 8 pixels, up to 3 above, bevel 8 pixels, up to 7 below; radius 8, half-width
25 — miter up to 8 above, bevel up to 16 below. The S-curve drew identical bytes in all three: at an
inflection the two chords' lags behind the tangent cancel and the chords barely turn.

## 2. Built

`Centre::tangent_continuous(j)`: at a point where a curve arrives or leaves, the path's own
arriving and leaving directions (a curve's recorded tangent, a line's chord) agree, and the join
there is round — §8.4.3.2's disc between the chords, as inside one curve (ADR 1455). **Agree means
equal to what `f32` can state of them**: each direction is a difference of two device points, each
within a few units in the last place of the largest coordinate `m` of the three points, so
`|v1 × v2| ≤ 4 · ε · m · (|v1| + |v2|)` with `dot > 0`. No angle larger than the arithmetic's own
uncertainty is swallowed: a turn of a tenth of a degree between two lines stroked 400 wide still
mitres (test). After the change the three styles draw identical bytes on all four fixtures at
half-widths 0.5, 3, 12 and 25 (failed before on the circle, watched), and the round join's ink is the
chords' distance set's to within a level per rim pixel.

Corpus, the same binary with and without (an environment switch, removed): 1×, 12 pages moved, all
12 equal to the oracle to four places (`comments`, `highlights`, `issue14415`, `pr12564`,
`rotated_ink` among them).

## 3. ADR 1443's lever for `issue14415.pdf`, derived and not built

The brief asked whether ADR 1375's tiling could run on the **coarse** flattening's pieces while the
fill takes the **fine** one. A tiling is not an oracle beside the fill; its fragments are the
geometry the fill integrates, so the fill would integrate the coarse set. And the coarse set is not
the fine one: every fine vertex lies on the curve, up to the coarse sagitta `s_c` beyond the coarse
chord, so the fine set's outer rim stands up to `s_c − s_f` past the union of the coarse pieces and
its inner rim that much inside it. Fragments of the coarse pieces therefore miss a sliver of the fine
set along every outer rim and cover points outside it along every inner rim — the rim error ADR 1455
section 4 measured at 4.6–9.0 levels a pixel. Using the coarse tiling only to say which fine pieces
are disjoint fails the same way: two fine pieces can overlap inside the sliver no coarse piece holds.
**It cannot be exact, so it was not built**; `issue14415.pdf`'s +50% stays, its lever still the tiling's
quadratic cut (ADR 1375), now with no cheaper construction left on the flattening's side.
