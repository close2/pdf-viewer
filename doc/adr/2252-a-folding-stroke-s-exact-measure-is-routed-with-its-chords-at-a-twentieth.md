# 2252 — A folding stroke's exact measure is routed with its chords at a twentieth of a pixel

Status: accepted for integration. Session 1709, 2026-10-23, the pixels slot of batch one hundred and
nine, under the project owner's answer `doc/questions/A559` ("I agree with 1/20 chords"). Clause:
ISO 32000-2 §10.7.4, with §10.7.2, §8.4.3.3, §8.4.3.5 and §8.5.3.3.2's non-zero rule; the row
stays `departed` for its other departures. Context: ADRs 1348, 1359, 1374, 2106, 2122, 2164, 2253.

## 1. What is routed

ADR 2122's construction, round 1664's patch rebased onto `ee365e4f` (it applied with offsets):
`render_cpu::area::union` finds the boundary of the set ADR 1348's pieces fill under the non-zero
rule, once for the whole mark, and `scan::mask_fill` accumulates it for `Exact::Pieces` under
anti-aliasing and the non-zero rule. `stroke::folding`'s `FOLD_FLATNESS` is a twentieth of a device
pixel where it was a sixty-fourth, and everything derived from it moves with it — the turn bound,
the pieces' overlap and the stretches' flattening. Three things are added, each found by rendering
the pages the change moves (trap 1) and each tested where it acts:

- **A mitre is the segments' own.** §8.4.3.5 converts a mitre to a bevel by "the angle φ between the
  segments", and at a vertex the path states that is its tangents' angle. The join was decided from
  the curve's first chord, which leans off the tangent by more as the chords coarsen: at a twentieth
  the collar of `annotation-fileattachment.pdf`'s push pin, a line arriving straight down and a curve
  leaving straight up (`φ = 0`), grew two mitres, 160 levels at 4x, and
  `pr12564.pdf` lost one the tangents admit. The chords' bevel is kept, since their bands' corners
  need it, and the mitre beyond it is decided, shaped and put on its side by the tangents:
  `a_mitre_is_decided_by_the_path_s_tangents_and_not_by_its_chords`.
- **A cap is turned from the path's tangent at its end**, as §8.4.3.3 draws it about the endpoint,
  not from a folding curve's last chord: `a_cap_is_square_to_the_path_s_tangent_and_not_to_its_chord`.
  A tangent is taken as a vector and never as a point beside the vertex: a curve's tangent runs to
  its nearest distinct control point, which may lie a part in a million away, and added to the
  vertex in `f32` it states no direction.
- **A boundary whose rows do not close is declined.** A horizontal line crosses a closed boundary as
  often upwards as downwards, so every row of its accumulator sums to nothing; the walk dropped a
  stretch on thirteen marks of the moved pages at 1x — `pr12564.pdf`'s nine drew streaks of 255
  levels across a glyph, `issue12823.pdf`'s one read its ink twelve levels short — and such a mark
  now goes to the library's converter before anything is written, as a walk that does not close
  already did (`a_boundary_with_a_stretch_missing_does_not_close`). Three marks are declined by the
  walk itself. Why the walk drops a stretch is not found: the round's dump of one such mark is
  `scratchpad/r1709/probe/`, and a round that finds it removes declines rather than adding any.

## 2. The gate, restated as the flattened set's

§10.7.4 measures a shape where "curves have been flattened to sequences of straight lines", and §10.7.2 leaves the flattening to this processor: "PDF processors may choose to
ignore any flatness tolerance specified within a PDF file." So:

- **The measure is held to the flattened set**: `a_folding_stroke_s_pieces_are_measured_as_their_
  flattened_set` draws the same pieces sixteen times finer, through `scan::mask_fill`, and holds
  every pixel of two rim windows to its block within the two drawings' roundings, one level. On the
  library's converter it is red, `(238, 231)` 208 against 202.5.
- **The paperclip is held to the curve by the flattening's bound**:
  `a_folding_stroke_covers_a_pixel_by_its_area` allows `255 · √2 · (1/20 + 1/320) + 1`, 20.2
  levels; the witness pixel reads 245 against 242.4 and the window's worst `(794, 566)` 61 against
  71.4. This one is not red on the library's converter (255 against 242.4 is inside the bound): the
  first test is the gate's edge.
- `stroke_wider_than_its_curve`'s ring, whose rim is the stroker's outline flattened to the chords'
  tolerance inscribed, is held to its rim times the twentieth: 1 658.9 against 1 661.9.

## 3. Priced

`callgrind_rasterise`, page 1, three draws, scale 1; HEAD `ee365e4f` against this tree. One thread:

| page | HEAD | this tree | |
|---|---|---|---|
| `inks.pdf` | 282 949 153 | 323 106 099 | +14.2 % |
| `rotated_ink.pdf` | 16 965 444 | 18 677 224 | +10.1 % |
| `issue14415.pdf` | 987 176 923 | 1 073 728 591 | +8.8 % |

At twenty-four threads, four runs a tree (one of HEAD on `issue14415.pdf`), the means are +13.7 %,
+2.3 % and +5.9 %, and the worst pairing +15.6 %, +11.3 % and +6.2 %; why a pool-width total is read
as a mean, and why no strip finds a mark's boundary twice, is ADR 2253. Every figure is under the
fifth, so the measure is routed.

## 4. The pixels that move

Twenty-six of the 974 first pages `raster_golden` holds move, raster only, and the goldens are
re-taken with this ADR; each is a page with a folding stroke. Its pixels moved and worst level at
1x: Cycle- and NoCycleType3insideType3 1071/39, 952/65; annotation-caret-ink 461/31, -fileattachment
8/50; asciihexdecode 296/54; bug1703683 650/27; bug1721218 80/21; bug1755507 2355/19; comments and
highlights 839/20; file_pdfjs_test 368/41; inks 4001/77; issue12810 682/21, 12823 2147/46, 13447
105/29, 14415 5153/52, 14438 159/50, 14953 54/32, 19182 182/20, 19360 5285/52, 20513 3055/30;
poppler-91414-0-53/-54 301/31; pr12564 916/80; rotated_ink 363/26; zerowidthline 393/33. Most are
the rims ADR 2106 measured the sixteenths' band on — up to 65 levels at 1x
(`ContentStreamNoCycleType3insideType3.pdf`) and 81 at 4x (`asciihexdecode.pdf`). Four moves are
larger, each rendered and read (trap 1):

- `inks.pdf` 4x, 228 levels: the red ink's end is a notch at a sixty-fourth, under either measure,
  and its round cap at a twentieth (`1 J`); the finer drawing reads the cap, `(799, 1454)` 230
  against 230.0. Where the union and the library part most on the same pieces, `(1637, 235)`, the
  union reads 205 against the finer drawing's 204.0 and the library 229: a rim pixel the sixteenths
  left near white.
- `pr12564.pdf`, 80 levels at 1x and 128 at 4x: the mitres of its stroked letters, which §8.4.3.5
  decides by the tangents, and nine of its marks, declined by §1's guard, drawn by the library.
- `issue14438.pdf` 4x, 178, and `issue20513.pdf` 4x, 122: a round-joined ink whose join the pieces
  lose at some chords and scales — at a sixty-fourth at 4x, holes in the stroke; at a twentieth at
  4x, the join drawn and the edge smooth, but at 64x, the finer drawing, lost again, so the finer
  drawing is no reference at those pixels. It is the pieces' construction (ADR 1348), not the
  measure, and §5 leaves it open.

The six arms' frame digests are HEAD's on every page, 0 of 5 799 — `render-raster` reaches
`render-cpu` only as its oracle — and the mean error between the frame and this oracle falls on
every moved page at 1x in all three lanes and at 4x in the processor's and the compute lane, 24 of
24, and on 20 of 24 in the device lane at 4x.

## 5. Decision

Routed. The §10.7.4 row's note states the flattened set's measure and drops the wait on
`doc/questions/Q559`. Left open, each with its witness, for a round on ADR 1348's construction:
the walk's dropped stretch (thirteen marks at 1x, declined; one dumped at `scratchpad/r1709/probe/`),
and a round join or cap the pieces lose at some chords and scales (`issue14438.pdf` `(1594, 1093)`,
`inks.pdf`'s red ink). Neither is new: HEAD draws both faults, at other scales.
