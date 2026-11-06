# 2735 — A JPX image's stated `/Indexed` space governs, and 17.2's screen `X` is the press's gamut

Status: accepted. Session 1960. Context: ISO 32000-2 §7.4.9, §8.9.5 Table 87, §8.6.6.3, §14.11.5, §10.8.3; ADR 0468
(a JPX image's stated `/ColorSpace`); the Ghent PDF Output Suite's patch 17.2 (ADR 2728), deficit 5 of
`doc/gwg-output-suite.md`.

## The suspicion

Round 1957 judged 17.2 failed in both views: its JPEG 2000 image draws a brighter green than the `X` behind it. It
suspected `image/jpx.rs` of drawing the codestream's colour rather than the dictionary's. §7.4.9 and Table 87 put the
dictionary first: "If present, it shall determine how the image samples are interpreted, and the colour space
specifications in the JPEG 2000 data shall be ignored."

## What the file states, and what is drawn

- **The image** is a JP2 file. Its one component indexes a one-entry `pclr` palette, (114, 247, 13), declared in
  enumerated sRGB.
- **Its dictionary** states `[/Indexed [/ICCBased <Adobe RGB (1998)>] 0 <72F70D>]`: the same three numbers, in Adobe
  RGB, as one entry.
- **The page** draws the image clipped to an `X`, over a square painted `0.701 0 0.997 0 k` under a PDF/X-4 intent.

This reader draws the `X` at (0, 247, 0). That is the Adobe RGB colour converted to sRGB: red and blue fall below zero
and clip, and green keeps its 247. The codestream's sRGB statement would have drawn (114, 247, 13). **The dictionary
governs, and the suspicion is refuted.** `jpx_enumerated_spaces.rs`'s
`a_stated_indexed_space_sets_the_data_s_palette_aside` pins the same rule on a fixture of its own. Its plant, the
palette applied under a stated space, fails it.

## The reading of the two views

- **Screen.** Adobe RGB's (114, 247, 13) lies outside what a CMYK press can print, and a screen shows it. The patch's
  `X` exists to reveal a conversion into the press. §14.11.5 makes the intent informational, and this reader draws an
  ICC colour by its own profile on a screen. So the screen's `X` is the press's gamut made visible, not a decoding
  defect. The verdict is **n/a**, as an overprint patch's is.
- **Press.** §10.8.3's simulation would convert the image through the press. The press view draws exactly what the
  screen draws, because `content::separate` answers `NotSeparated::Nothing` on a page that names no spot colourant.
  That is deficit 5, and 17.2's press verdict stays a failure under it.

## Decision

- No change to `image/jpx.rs`. One test pins the route.
- `doc/gwg-output-suite.md` records 17.2 as n/a on screen and as a press failure under deficit 5. Deficit 3 is struck
  from the ranked list with this reading. The harness keeps both comparisons, failing, under names that say why.
