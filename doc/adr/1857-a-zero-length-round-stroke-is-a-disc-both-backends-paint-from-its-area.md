# 1857 — A zero-length round stroke is a disc both backends paint from its area

Session 1510. Status: **accepted** and **built**. `doc/todo/69` §1's dot, for the project owner's
performance target of 2026-10-10. Context: ADRs 0290, 0401, 0420, 1374, 1817, 1829, 1835, 1847;
raster's ADR 0097; traps 1, 2, 13. Code: `crates/pdf-render/src/thin_segment.rs` (`thin_segment`),
raster's `encode/line.rs` (`pen`, `encode_line_as_stroke`), `raster/stroke.rs` (`dot_pieces`),
`shaders/line.wgsl` (`unit`), raster-scene's `check_line`. Tests: `render-cpu/tests/
thin_segment_ladder.rs` (two), raster's `tests/line_lane.rs` (two), `pdf-render`'s and
`encode::line`'s units. Clauses: §8.5.3.2, §8.4.3.2, §8.4.3.3, §10.7.4. No ledger row moves.

## 1. The premise, and the hypothesis

The premise held in the code: `thin_segment` refused `from == to`, so the witness's 28 684
`m x y l x y S` dots under `1 J` went through `split_degenerate`, and on the device each was a filled
circle path with an outline of its own. The hypothesis held as stated: in the frame where the pen is
the unit disc, the capsule of length zero is the disc, and both kernels already measured it. The
band's four corners collapse to two points and clip to no area; the two half discs about one point,
facing any opposite pair of directions, are the disc. Nothing new is measured. What each kernel
lacked was a direction: the processor's `Frame` already took (1, 0) at length zero, the shader's
`normalize` of zero is not a unit vector.

## 2. The decision

**§8.5.3.2's dot joins the thin segment's population under a round cap**: "the S operator shall paint
it only if round line caps have been specified, producing a filled circle centred at the single
point." The filled circle is that set at length zero, so both backends paint it from the closed form,
and it never reaches `split_degenerate`'s circle of four cubics. Under a butt or projecting cap the
dot stays declined, and `split_degenerate` gives the clause's "no output".

**A dot none of whose pixels would hold one level stays with `point_mark`**, on every backend. The
processor's floor for a thin segment (ADR 1374's, through ADR 1829) lifts every pixel such a mark
touches. At a pixel's corner that is four levels for a mark of two. The device's line lane has no
floor and would round each quarter to nothing. `point_mark` places the mark whole in the pixel
§10.7.4 names by flooring its centre, so its ink does not depend on the grid
(`sub_pixel_coverage.rs`'s `a_dot_lands_in_one_pixel_at_every_width_and_every_placement` named
this, failing at 0.1 of a pixel on a corner with 0.01569 of ink against 0.00785). So `thin_segment`
takes a dot's areas, and only a dot's, and admits it where one pixel holds a level. A segment's
classification is the block's four comparisons as before. The witness's dots are 0.143 of a
pixel across, four levels, and every one is admitted.

**The device draws it as one line instance.** The scene admits equal points under a round cap.
`pen` keeps the pen for them. The shader takes (1, 0) where the two ends coincide. Where the closed
form does not apply, a residue clip or a turned transform, the stroke road expands nothing for a
subpath with no length. There the dot is the stroker's own two round caps about the point,
stretched to the pen's semi-axes (`dot_pieces`), and is met with the clip like any outline.

## 3. Evidence

- **The ladders.** The processor: every dot of seven widths from 0.047 to 1 pixel and 81 placements,
  both axes, and the (½, ¼) ellipse, against the ladder's own integral of the set's chords. Every
  dot whose largest pixel holds a level is admitted, and they number more than 972. Butt and
  projecting dots are declined and paint nothing. Strips of one, three and seven rows paint the
  dots' bytes. The device, `line_lane.rs`: 525 dots under the identity, unequal scales and a
  quarter turn, on RADV and llvmpipe, `Cpu` and `Compute`: worst 0.602 and 0.500 levels against an
  inscribed 4 096-gon. A dot under a residue is within 1.149 levels of the circle met with the
  clip, and its ink is 853.0 against 850.4. Calibrated: the shader's direction put back to
  `normalize` fails at 255 levels; `dot_pieces` emptied fails at no ink; a 2 % deficit in the disc's
  area fails the processor's rung. Each was reverted.
- **The witness at the fit** (1416 × 1000), HEAD's export against this one's. The device uploads
  **4 107 resources where it uploaded 32 791**: the dots were 28 684 outlines. Device against
  oracle: bytes differing 649 717 → 617 668 on `Cpu` and 558 710 → 553 396 on `Compute`; worst
  15 levels in both. The processor's page: 37 427 pixels move, at most 13 levels, ink +0.044 %. Looked
  at: each dot, drawn before as a one-pixel circle at a twentieth of the alpha, is now the disc's own
  area, crisper. The shipped window against the same window in one strip: 142 → 63 pixels at the fit
  and 91 → 73 at 2×. The dot's pixels no longer depend on the cut.
- **`raster_golden`**: held 973, moved 1. `issue14953.pdf` moves 6 pixels by one level, the dots of
  a stroked font's joints (looked at). The division test is unchanged at 176 pages, ≤ 1 308 pixels, ≤
  3 levels, and it says `pdfjs_wikipedia.pdf` may leave its named list.

## 4. What is left

A single-point *closed* subpath (`m h`) and a dot inside a longer path are still
`split_degenerate`'s. They are a census away from the same treatment, and nothing on the witness
states one. The ledger row's population sentence (97 dots over 7 first pages, the thinnest 0.36 of a
pixel) stands; those dots are admitted now.
