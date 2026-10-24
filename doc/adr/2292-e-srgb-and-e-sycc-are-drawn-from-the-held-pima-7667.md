# 2292 — e-sRGB and e-sYCC are drawn from the held PIMA 7667

Session 1729, the clause slot of batch 112. Status: implemented; §7.4.9 stays `partial`, its
reason narrowed to CIEJab (19) alone. Builds on ADRs 1383 and 1713; ADR 2187's remaining build is
this one. Context: ISO 32000-2 §7.4.9; ITU-T T.801 Table M.25 and M.9.2.4 (held); PIMA 7667:2001,
bought by the owner (`doc/questions/A348`), provenance in `doc/third-party-data.md`. Code:
`crates/pdf-colour/src/colour/extended_srgb.rs` (new), `crates/pdf-model/src/image/jpx.rs`
(`jpx_drawn`, `JpxReading`, `extended_on`). Tests: `crates/pdf-model/tests/jpx_enumerated_spaces.rs`
and the module's own.

## 1. The reading

T.801 Table M.25 gives `EnumCS` 20 to e-sRGB and 24 to e-sYCC and defines neither, handing both to
PIMA 7667 (the brief's 21 is ROMM-RGB, PIMA 7666, already drawn by the codec). PIMA 7667 defines
e-sRGB on a reference display (section 4.3): BT.709-3's primaries, D65's chromaticity at 80 cd/m²,
and — section 4.4.1 encodes a flareless display — zero tristimulus at zero code value. Decoding is
its section 4.5: equation 5 removes equation 4's offset `2^(n−2) + 2^(n−3)` and equation 3's scale
`255 × 2^(n−9)`; equations 6a to 6c invert the sRGB curve, extended oddly about zero; equation 7 is
the matrix to tristimulus values with white's `Y` at 1. e-sYCC is Annex B.2: B7 and B8 to
fractions, B9's matrix to e-sRGB's nonlinear components, then section 4.5. Four decisions sit on
that reading:

- **Precisions.** Section 4.1 defines e-sRGB10, e-sRGB12 and e-sRGB16 and no other `n`, so a box on
  a codestream of another depth names no encoding the standard defines and takes §7.4.9's next
  specification or its fallback. Annex B puts no limit on e-sYCC's `m`, so 1 to 16 bits are drawn.
- **B9 as printed.** B.2's transformation is the printed matrix, `0.0003` included, not B4's
  inverse, which differs in the fourth place.
- **B10 is not taken.** It requantises to an n-bit e-sRGB integer that equation 5 then divides out
  again; the e-sYCC file states no `n`, and what the step would change is half an n-bit level.
- **Luminance outside 4.3.4's bound.** The section says an e-sRGB colour shall not exceed the
  white's luminance nor go below zero. A colour that does is drawn with its lightness held at
  `Lab`'s 0 or 100 by §8.6.5.4's range — a choice for data the standard forbids, recorded here so
  a fixture is not built on it (the first e-sYCC fixture of this round was, and was replaced).

## 2. The route to the connection space

The integers reach this crate untouched: the choice makes every `colr` box a Free box, as it does
for a space the codec does not draw, and remembers the encoding. The XYZ under the reference white
is adapted onto D50 by the Bradford route every CIE-based space here takes, and painted as §8.6.5.4's
`Lab` under D50 with unbounded `a*` and `b*` (`ExtendedSrgb::connection_space`). `Lab` and not
`CalRGB` because `CalRGB` clamps its components to 0 to 1, which would clip an extended colour in
the source before an output intent or a blending space could use it. PIMA 7667's informative Annex
E.2 recommends the same linear Bradford adaptation for an e-sRGB profile, and its E3 matrix agrees
with this composition to a unit in its third printed place.

## 3. What checks it

Expected pixels are worked from the standard's own equations, not from another reader (principle
5): the 16- and 12-bit e-sRGB fixtures against Annex A's A2 with A.3's clipping, an 8-bit e-sYCC
fixture against B7 to B9, and the module against section 4.4.3's neutral-patch table at all three
precisions. The conversion is upstream of every rasterizer — the image reaches the oracle and the
device as one RGBA raster — so the lanes cannot part over it; the corpus arms stand byte for byte,
which no box in reach exercises (`examples/jpx_colour_census`, ADR 1510).

## 4. What would reopen it

A document in the census stating 20 at an undefined depth, or the owner reopening CIE 131 for 19.
