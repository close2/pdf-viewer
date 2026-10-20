# 2086 — A replacement is one character of the accessibility tree's line

Session 1625, slot 3 of batch ninety-eight, a host round. Status: **accepted** and **built** in
`crates/viewer-core/src/viewer/geometry.rs::device_lines`, tested by
`crates/viewer-core/tests/tagged_copy.rs::a_replacement_over_two_codes_is_one_character_of_its_line`.
Context: ADR 1515 (every code inside an `/ActualText` section reads back as the whole replacement),
ADR 2078 and `doc/history/1621` "Found, not fixed" (1). Clause: ISO 32000-2 §14.9.4.

## 1. The defect

§14.9.4: "The ActualText value shall be used as a replacement, not a description, for the content".
`pdf-model`'s `push_replacement` gives each code the section showed the span of the whole
replacement, which is ADR 1515's decision and what a selection and a caret rest on. `device_lines`
appended each code's slice of the readback to the line, so a section of two codes was spoken twice:
the standard's own EXAMPLE, `(Dru) Tj /Span <</ActualText (c)>> BDC (k-) Tj EMC`, read `Der Drucc`
in the accessibility tree. `TextLine`'s rule — its text is exactly its characters' readback joined —
held by byte count and failed by content, which is why the census's invariant did not see it.

## 2. Decision

**The fix is in the line, not in the readback.** A code whose span is the span the previous
character of the line already spoke is the same replacement; its glyph's box is added to that
character's box and nothing is appended. So the section is one `Character` whose `bytes` is the
replacement's length and whose `bounds` covers every glyph it stands for — the example's first line
is `Der Druc`, eight characters, the last as wide as `k-` together.

Changing `push_replacement` instead was the alternative, and it is the worse one: the shared span is
what makes a selection or a caret anywhere inside the section find the whole replacement (ADR 1515),
and narrowing it to the first code would make the second code a position with no text.

## 3. Costs

A caret cannot stand between the two glyphs of a replaced section. That is the clause's own unit —
the replacement is "a character substitution for the content enclosed", not one per code — and a
platform asking the extent of the character gets the whole of what it replaced. A section whose codes
fall on two lines is spoken once per line; no fixture states one and none is invented here.
