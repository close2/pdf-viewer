# Q687 — May a font that names a face and states no widths be given the layout this viewer draws?

Asked by round 1821, the archive slot of batch 122 (ADR 2477). Nothing below is built; the line it
asks about is yours (`doc/questions/A48`: "state an interpretation the standard defines; never fill
in an absence").

**What the ranking found.** With the CJK pack named, the largest reason that is neither a profile's
`stop`, a fence row nor the press profile is `fonts/font-programs-embedded` refusing a font that
names a face outside ISO 32000-2 section 9.6.2.2's fourteen and states **neither a
`/FontDescriptor` nor `/Widths`**: ten documents at each part and under both profiles. Table 109
lets only the standard 14 leave both out, and Annex D publishes metrics for those; for `/Arial`,
`/Tahoma`, `/Monaco` or `/TimesNewRoman` the standard states no layout, so the sentence reads "no
table of the standard's says what it is laid out by".

**What lifting the refusal would do.** The viewer already draws such a font from a shipped face and
lays it out by the metrics of the standard font its request resembles (`pdf_font::standard_metrics`,
which maps `/Arial` to Helvetica's "in practice"). Allowing the route that writes a descriptor and
`/Widths` for the standard 14 (ADR 2146) to take any simple font's name was tried in an export, with
the readback proof (`described::proved`) held to the viewer's own drawing:

- eight of the ten convert under `as-if-printed` at 2b and at 4: `issue6069`, `issue6106`, `issue6108`
  (`/Arial`), `issue18099_reduced` (`/Tahoma`), `issue2391-2` (`/Monaco`), `All Quiet on the Third
  Coast` (`/TimesNewRoman`), and two hand-built corpus files (`/Wingdings`; a font with no
  `/BaseFont` at all);
- veraPDF 1.30.3 passes fourteen of the sixteen outputs; the two it fails are the file with no
  `/BaseFont`, whose descriptor would have to invent a `/FontName` (test 6.2.11.2-3);
- the two that stay refused are a font whose `/Subtype` is `/Type7` and a font whose descriptor is
  spelled `/GontDescriptor` and states its own widths.

**The cost.** The widths written are this viewer's layout of a face the file never described: another
reader (Acrobat draws `/Tahoma` from a Tahoma, whose advances are not Helvetica's) would have laid
the line out otherwise, and the output would state this reader's choice as the file's. That is the
filling-in of an absence A48 names; it is the same shape as ADR 2146 but without the standard's
published metrics under it.

*Recommendation*: **keep the refusal**, and say so in its sentence (it now reads "not built yet",
which is not true; ADR 2477 proposes rewording it as a fence row citing A48). If you want the eight,
the route is a loss of its own (a name such as `layout-from-this-viewer`) that `as-if-printed`
discards and `as-shown` leaves at `stop`, so the guess is a stated loss and never a silent one, and a
font with no `/BaseFont` stays refused.

**Options**: (a) keep the refusal and reword it as the fence (recommended); (b) lift it behind a new
named loss as above; (c) lift it with no loss, which this round advises against.

**Meanwhile**: the refusal stands unchanged under its present sentence, and nothing is built for these ten documents until you answer.
