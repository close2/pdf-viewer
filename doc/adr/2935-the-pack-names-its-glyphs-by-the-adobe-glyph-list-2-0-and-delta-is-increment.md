# 2935 — The pack names its glyphs by the Adobe Glyph List 2.0, and `Delta` is U+2206

Session 2060. Status: **accepted**; Q888 asks the one departure it would take. Context: ADR 2630's requirement 3,
ADR 2934, ISO 32000-2 §9.6.5 and §9.10.2.

## 1. The question

A simple font reaches a glyph of a bare CFF program by its name (§9.6.5), so the pack's glyph names decide which
`/Differences` names it answers. ADR 2630's plant named Noto's Greek capital delta `Delta`. The Adobe Glyph List
2.0, the list §9.10.2 sends a reader to and the one `read-fonts` and fontTools carry, maps `Delta` to U+2206
INCREMENT and names U+0394 `Deltagreek`; likewise `Omega` is U+2126 and `mu` U+00B5. Version 1.x of the list
mapped some names to two values; 2.0 does not.

## 2. The decision

The builder names each glyph by the list as it stands: every name 2.0 gives the glyph's character, and nothing it
does not. Noto Serif maps no U+2206, so the pack has no `Delta`, and a font whose page shows `/Delta` keeps its
refusal or its photograph. Giving `Delta` the Greek letter's outline would pick a glyph for a character the face
does not state, from a mapping the current list withdrew; that is the choice Q888 puts to the owner rather than a
reading this round can derive.

## 3. Consequences

The Greek document of ADR 2630 (`Deficient inflammasome activation …`) stays photographed under `as-if-printed`:
its `Times-Roman` shows `/Delta` at code 135, which the Times-Roman metrics name at width 612, and every other code
the pack draws. `mu` reaches U+00B5's glyph and `Omega` U+2126's, both present in Noto Serif.
