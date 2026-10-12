# 1875 — Every degenerate subpath is the disc, and a path's dots are discs where no two share a pixel

Session 1519. Status: **accepted** and **built**; the device's half is named in §4.
ADR 1857 §4's remainder, for the project owner's performance target of 2026-10-10. Context: ADRs
1829, 1835, 1857; traps 1, 2, 13. Code: `crates/pdf-render/src/thin_segment.rs` (`segment_ends`,
`thin_dots`, `ThinDots`), `crates/pdf-render/src/degenerate.rs` (`subpaths`, now the crate's),
`crates/render-cpu/src/thin_segment.rs` (`dots`, `paint_segment`, `paint_dots`) and its call in
`render-cpu/src/lib.rs`'s `draw_stroke`. Tests: `render-cpu/tests/thin_segment_ladder.rs` (two),
`pdf-render`'s units (two). Clauses: §8.5.3.2, §8.4.3.2, §11.3.6. No ledger row moves; §8.5.3.2's
row gains the tests and a sentence.

## 1. The premise, and what the census found

The premise held in the code: `thin_segment` matched `[m, l]` alone, so §8.5.3.2's "single-point
closed path" — `m h`, `m l(p) h`, a curve whose three points are the start — and every dot inside a
longer path went through `split_degenerate`'s circle of four cubics and `tiny-skia`'s fill. A census
of 958 first pages of `doc/pdf.js/test/pdfs` at scale 1, round caps (round 1519's scratch example):
`m l(p)` 39 strokes on 1 page, 5 admitted; `m h` 27 strokes on 2 pages (`issue12810.pdf` 25,
`issue6961.pdf` 2), 25 admitted; a dot inside a longer path 1 stroke on 1 page (`issue13211.pdf`),
past the kernel's two-by-two block, so 0 admitted. The witness states none.

## 2. The decision

**One segment is one function's answer.** `pdf_render::segment_ends` reads `[m, l]` as its two
points and any one-subpath path whose every command stays at its move — lines and curves to that
point, a close only as the last — as the segment from the point to itself, which ADR 1857 made the
disc. `thin_segment` asks it, so `m h` is classified as `m l(p)` is, and a backend that sends a
segment's own points can ask the same function rather than restate the slice pattern (trap 2).

**A path's dots are discs only where they are its only marks in their pixels.** §8.4.3.2 makes a
stroke one set, and one fill of the clause's circles paints their union: a pixel two circles share
holds the union's area. Two composites of two discs put `1 − (1 − a)(1 − b)` there, which is less.
So `thin_dots` takes a path's dots out of it when every one is admitted by `thin_segment` and no
two of their blocks share a pixel. Then each pixel holds one disc's area either way, and the two
constructions paint one set. Any other path is declined whole, because admitting some of its dots
would bring the same question back between the two groups. The rest, with a closed subpath's
successor given the move its close implied (Table 58), is stroked first. The dots are painted
after it, the order `split_degenerate`'s callers already draw the two in.

## 3. Evidence

- **Ladder rungs, calibrated.** `m h`, `m l h` and `m l` paint the same bytes at 81 placements and
  two widths. One stroke of a rule and 81 dots paints the bytes of the rule and the dots as separate
  strokes. With `segment_ends` refusing the closed spellings the first fails on its classification.
  With `thin_dots` answering `None` the second fails on the bytes. Each was reverted.
- **`raster_golden`.** `issue12810.pdf` moves (with ADR 1876's pages). Its worst pixel, 11 levels
  darker at (897, 723), is a dot drawn as the disc's area where `tiny-skia`'s fill sampled the
  circle. Looked at beside HEAD's.

## 4. What is left

**The device takes `m h` through `render-raster`'s `thin_line`**, which asks `segment_ends` in the
same batch; that file was session 1520's, and the swap was asked of it by message. The
device keeps `split_degenerate`'s circles for dots inside longer paths, which 0 admitted dots on the
census reach. What either leaves between the lanes is the device's fill of four cubics against the
processor's closed form, inside raster ADR 0082's bound.
