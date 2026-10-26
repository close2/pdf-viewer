Status: complete
Given: 2026-10-26, in conversation — transcribed by the round
Owes: the build — the accessibility node gains what Table 380 states and a sighted reader sees
drawn: `/TextDecorationType` through `accesskit`'s `TextDecoration` (underline, overline,
strikethrough), `/BaselineShift` read as sub- or superscript by its sign (a mapping this program
chooses and documents, since §14.8.5.4.4 states a distance and not a meaning), and the ruby
attributes where the platform role exists; the row moves to `partial` naming the three it leaves
(`/LineHeight`, `/GlyphOrientationVertical`, `/TextPosition`, which no platform counterpart
carries); no reflowing view (acting round)

> Otherwise I agree with all recommendations.  (only departures: XFA and Q632, which I haven't
> fully understood yet)

Reading: one message answered six questions together and named its two departures; this one
adopts Q634's recommendation as presented, quoted and attributed: *"Yes, for `/TextDecorationType`,
`/BaselineShift` read as sub- or superscript by its sign (a mapping this program would choose
and document, since the clause states a distance and not a meaning), and the ruby attributes
where the platform role exists."*

What the answer settles:

- **The row's recorded reason had decayed and the decay is acted on**: `inapplicable` rested on
  "exports to no format", and `viewer-core`'s `accessibility.rs` now hands the structure tree to
  the platform's assistive technology — so the inline attributes enter the export, one round,
  no dependency, nothing on the launch path (the tree is built after the first frame).
- **The mapping is a documented choice, not a derivation**: `/BaselineShift` states a distance;
  sub- and superscript are what `accesskit`'s `VerticalOffset` carries, and the choice of reading
  the sign that way is written down as the choice it is.
- **The reflowing view stays out**, which is what the four neighbours' attributes serve: that is
  a layout engine, on the authoring exclusion's side of the line.