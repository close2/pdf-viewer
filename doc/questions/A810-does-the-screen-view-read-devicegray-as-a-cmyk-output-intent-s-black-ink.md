Status: complete
Given: 2026-11-09, in conversation — transcribed by the round
Owes: read DeviceGray (when no /DefaultGray is present) as the CMYK output intent's black ink in the screen view whenever the page's selected intent is a CMYK profile regardless of subtype (option 1, one condition in output_intent::grey_reading), with a corpus census of CMYK-intent files with grey content and GWG patch 23.0's screen checks updated to pass (a colour / model round)

> Q810: Option 1

Reading: adopts option 1 — in the screen view, read `DeviceGray` (with no `/DefaultGray`) as the CMYK output intent's black ink whenever the page's selected output intent is a CMYK profile, whatever its subtype, so PDF/A and PDF/X behave alike, `0 g` and `0 0 0 1 k` match on screen, and GWG patch 23.0 passes in the screen view; backed by a corpus census of CMYK-intent files holding grey content.
