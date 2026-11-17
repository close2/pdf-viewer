# Q888 — May the Latin-Greek-Cyrillic pack draw the glyph name `Delta` with Noto's Greek capital delta?

Source: round 2060 (ADR 2934, ADR 2935). Nothing waits on the answer: the pack is built, and a font showing
`/Delta` keeps the refusal or the photograph it has today.

## What was found

ADR 2630's Greek document draws one code, 135, through the glyph name `Delta` in an unembedded `Times-Roman`. The
Adobe Glyph List 2.0 — the list ISO 32000-2 §9.10.2 names — maps `Delta` to U+2206 INCREMENT, and names the Greek
letter U+0394 `Deltagreek`. Noto Serif has no U+2206. So the pack, built by the list as it stands, has no `Delta`,
and the document stays photographed under `as-if-printed` at both parts for that one code; every other code it
shows is drawn. Version 1.x of the list mapped `Delta` to both U+0394 and U+2206, and the standard 14 metrics
name the glyph `Delta` in `Times-Roman` without saying which.

## The question

May the pack's builder give the name `Delta` (and `Omega`, `mu`, the other names 1.x mapped twice) a copy of the
Greek letter's glyph where the source face has no glyph for the 2.0 value?

## Recommendation

**No, for now.** INCREMENT and the Greek capital delta are separate characters with no Unicode decomposition
between them; drawing one for the other is a choice of glyph the face does not state, made from a mapping the
current list withdrew. The document keeps its photograph, which shows the page as the window draws it. If you
want it, the builder's change is one table of the double-mapped names, and the pack's digests move.
