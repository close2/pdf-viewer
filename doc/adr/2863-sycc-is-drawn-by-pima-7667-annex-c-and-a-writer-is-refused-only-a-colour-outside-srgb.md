# 2863 — sYCC is drawn by PIMA 7667's Annex C, and a writer is refused only a colour outside sRGB

Round 2023, the ledger lane.
Status: **accepted and built** in `pdf-colour` (`colour/extended_srgb.rs`: `ExtendedEncoding::SrgbYcc`,
`ExtendedSrgb::nonlinear`, `ExtendedSrgb::within_srgb`) and `pdf-model` (`image/jpx.rs`: `jpx_drawn`, `Sycc`,
`jpx_samples`, `every_pixel_within_srgb`).
Carries out ADR 2793's decision 2. Builds on ADRs 2292, 2293 and 2794.
Clauses: ISO 32000-2 §7.4.9; ITU-T T.801 Table M.25 (18); PIMA 7667:2001 Annex C (normative).

## The reading

ADR 2794 read T.801's 18 as the encoding PIMA 7667's Annex C converts. Annex C.2: "The conversion from m-bit sRGB
YCC to n-bit e-sRGB shall be as follows". C7 is `Y″ / (2^m − 1)`. C8 gives each chroma as
`(C″ / (2^m − 1) − 0.5) / 2`. Then "equations B9 through B11 are applied from Annex B". The annex opens with the
reason: the encoding "potentially spans a larger gamut than sRGB". The codec clipped that range before this
crate saw the raster, so a destination other than the screen received a colour that had already been clipped
(ADR 2793).

## The construction

- `ExtendedEncoding::SrgbYcc` reads C7 and C8 and then shares e-sYCC's B9 and section 4.5. Its colours go to
  the connection space as `Lab` under D50, as 20 and 24 do (ADR 2292), and no colour is clipped before the
  destination. Every depth from 1 to 16 bits is drawn, since Annex C fixes no `m`. A codestream whose three
  components do not share one unsigned depth is still drawn by the codec's matrix, because C7 and C8 are written
  for one `m`.
- **A writer is refused only what it would clip.** ADR 2793 held this build back because ADR 2293's refusal would
  reach every sYCC image a writer carries. The annex itself narrows that. Of C.2 it says "there will be an exact
  correspondence between the sRGB image data and the e-sRGB image data" where the gamut used is sRGB's. So
  `jpx_samples` checks every pixel. If all lie inside sRGB's gamut, the image is decoded again by the codec's
  matrix and carried as RGB, as before, and ADR 2794 found the two agree to a level there. Only an image with a
  colour outside the gamut is refused, by name.
- **The margin is derived, not chosen.** An integer stands within half a code of the colour encoded, so a
  chroma's uncertainty is `0.5 / (2^m − 1)`: C8's halving applied to a half-code rounding, with C6's and C8's
  centres half a code apart beside it (ADR 2794's decision 3). B9 sums the luma's error and each chroma's
  times its coefficient. The margin on each nonlinear component is therefore `0.5 × (1 + Σ|chroma
  coefficients of its row|) / (2^m − 1)`. That is 0.0075 on `R′` at eight bits, and it admits C5 and C6's own
  white, whose `R′` decodes to 1.0027.

## Consequences

- §7.4.9 owes nothing but CIE Jab (19), which A348 declines. The row is `departed` on 19, as ADR 2793's
  decision 2 set out, and §7.4 follows (ADR 1599).
- ROMM-RGB (21) stays drawn by the codec; the owner declined PIMA 7666 (A825).
- On the screen, a sYCC colour inside sRGB's gamut draws as before, within a level. A colour past the display
  white's luminance now takes ADR 2292's reading of section 4.3.4: `L*` is held at 100 and `a*`, `b*` are kept.
  The codec clipped each component instead. `sycc_is_drawn_as_pima_7667_annex_c_converts_it` works both by hand,
  and (255, 128, 96) now draws (203, 255, 248) where the codec drew (211, 255, 255).
