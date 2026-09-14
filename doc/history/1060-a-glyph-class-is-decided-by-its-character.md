# 1060 — A glyph class is decided by the character, because a substitute is reached by nothing else
`/FD` was read and applied to nothing for 559 sessions, on the argument that a class's CIDs are the
collection's. They are — but the *characters* are here, and Table 123 describes every class by them.

## The reading
Table 122: a class's descriptor's entries "shall override the corresponding values in the main font
descriptor dictionary for that class of glyphs". The one such value a page can see here is which
installed face stands in for a font the document did not embed (ADR 0007). §9.7.4.2 leaves a
non-embedded CIDFont reachable by nothing but §9.10.2's Unicode value — "In this case, CIDs shall
not participate in glyph selection" — so every glyph a descriptor can decide arrives with its
character in hand, and Table 123 names each class by the characters its glyphs stand for:
"Proportional Latin glyphs", "Full-width hanzi (Chinese) glyphs", "Hangul and jamo glyphs". No CID
is read; Table 116's boundary is untouched.

`glyph_class::Decision` is the reading; `substitute::overridden` merges a class's descriptor over the
main one, dropping the seven keys §9.8.3.3 says it shall not hold, which also stops `/FD` recursing;
`loading::class_faces` derives a second `Request` and face per class; and `LoadedFont` carries
`Placed { face, glyph }`, because a cache keyed by a glyph index alone would hand one face's contours
back for another's glyph.

## What is refused, by name
`LoadedFont::refused_glyph_classes`, from Table 123's own words: the `Rot` classes ("Same as HRoman
but rotated for use in vertical writing") and `Ruby` separate a class from another by the glyph's
*form*; `Dingbats` ("Special symbols") and `Generic` ("Typeface-independent glyphs, such as
line-drawing") by nothing a Unicode value states; plus a class Table 123 does not list for the stated
collection, an ordering it does not tabulate (every `Identity` one), and two stated classes that
could hold one character — `Proportional` against `HRoman`. A refusal keeps the main descriptor.

## The census, and why the fixture is the only witness
`pdf-model/examples/glyph_class_census` over 91 800 documents: **129** font descriptors of 614 068
state `/FD`, in 32 documents; each names exactly one class (`Proportional` 123, `Alphabetic` 6); 127
embed their program. All 129 restate `/Flags` and `/ItalicAngle` unchanged and none states a
`/FontWeight` — the three entries a face is chosen by; what they override is `/StemV` (128),
`/StemH` (126), `/CapHeight` (109), `/XHeight` and `/Ascent`/`/Descent` (108 each), none an input
here. So no document on this disk can move a glyph through this route, `raster_golden` held 974 of
974, and the fixture is the witness — calibrated (trap 13) by deleting the `class_face` arm: its
first assertion fails, its second passes.

ADR 1074 has the rule, including the one that keeps it quiet: a class whose merged descriptor derives
the request the main one already derives has stated no exception, so no second face is looked for.

**Rows.** §9.8, §9.8.3 and §9.8.3.1 `partial` → `implemented`, each having been `partial` for `/FD`
alone; §9.8.3.3 stays `partial` for the classes no character decides.
