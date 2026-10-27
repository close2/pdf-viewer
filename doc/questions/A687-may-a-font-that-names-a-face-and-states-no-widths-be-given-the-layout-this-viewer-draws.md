Status: complete
Given: 2026-10-27, in conversation — transcribed by the round
Owes: lift the refusal in `described::describe` for a simple font outside the standard 14 that states neither `/FontDescriptor` nor `/Widths` behind a named loss (`layout-from-this-viewer`) that `as-if-printed` discards and `as-shown` leaves at `stop`, and supply a conforming `/BaseFont` / `/FontName` when `/BaseFont` is absent (or cover the `/GontDescriptor` / `/Type7` typos where the viewer draws the page) so the viewable documents convert and pass veraPDF (an archive round)

> Q687: I agree with option (b) — lift the refusal behind a named loss (`layout-from-this-viewer` that `as-if-printed` discards and `as-shown` stops on), and also supply a valid `/BaseFont` name for the document that omits `/BaseFont` so it passes veraPDF.

Reading: overrules the round's recommendation (a) and adopts option (b), consistent with `A681` (1): if the viewer can display the document, the archive converter should convert it under a profile by recording the loss explicitly (`layout-from-this-viewer`, discarded by `as-if-printed` and stopped on by `as-shown`). Where `/BaseFont` is omitted on a font the viewer draws, supply a valid `/BaseFont` name matching the embedded descriptor's `/FontName` so the output passes veraPDF 6.2.11.2 test 3 as well.
