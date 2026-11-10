# 2794 — sYCC is held to PIMA 7667's Annex C

Session 1991, a ledger round (§7.4.9). Status: implemented (a test). Row: §7.4.9. Builds on ADRs
1510 and 2292.

## Context

T.801 Table M.25 says enumerated colour space 18 is "sYCC as defined by IEC 61966-2-1, Amd.1",
which this project does not hold. Until now the only test was that a colour with no chroma is
neutral (`sycc_with_no_chroma_is_neutral`), because nothing held fixed the matrix.

PIMA 7667:2001 is held (A348), and its Annex C is normative. Its foreword lists only Annexes D to
H as "for information only". The annex is titled "Conversions between e-sRGB and sRGB YCC
encodings". It names the encoding as "[t]he sRGB based YCbCr color encoding consistent with ITU-R
BT.601-5, which is specified in a proposed amendment to ISO/IEC 15444-1 as sRGB YCC", and its C.2
says "[t]he conversion from m-bit sRGB YCC to n-bit e-sRGB shall be as follows": C7 is `Y″ /
(2^m − 1)`; C8 gives each chroma as `(C″ / (2^m − 1) − 0.5) / 2`, which is e-sYCC's half-scale
chroma; and then "equations B9 through B11 are applied from Annex B".

## Decision

1. **The encoding Annex C converts is read as T.801's 18.** This is a reading, and its basis is
   the held texts. Both name the sRGB-based YCbCr that ISO/IEC 15444-1 took up as its YCC space.
   PIMA 7667 (2001) names it by its BT.601-5 matrix over sRGB, and T.801 (2002) names it by IEC
   61966-2-1 Amd. 1. The reading would be revisited if that amendment were ever held and differed.
2. **`sycc_is_drawn_as_pima_7667_annex_c_converts_it` works C.2 by hand.** It uses four pixels
   inside sRGB's gamut and two outside it, and on this screen a component outside 0 to 1 is
   clipped, as A.3's NOTE says. The codec agrees to one level everywhere.
3. **The half level is the text's own.** C6 encodes a chroma about `2^(m−1)` (128 at eight bits),
   while C8 decodes it about `(2^m − 1) / 2` (127.5). The test's one-level tolerance holds either
   centre, and it says so. Choosing between them would be choosing for the text.
4. **No code moves.** The codec's matrix is held, not replaced. Where the codec differs from Annex
   C, which is a colour outside sRGB before a destination other than the screen, ADR 2793 records
   it as the row's owed reason.

## Plant

Routing 18 through e-sYCC's equations (`E_SRGB | E_SYCC | 18` in `jpx_drawn`) draws the second
pixel's red at 212 rather than 182. The test fails. Reversed, it passes.
