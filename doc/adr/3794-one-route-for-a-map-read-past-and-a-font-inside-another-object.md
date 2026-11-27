# 3794 — One route for a map read past, and a font written inside another object

Session 2273. Status: **accepted** and built. Context: ADR 2294 (placeholders taken out), ADR
2608 (a font written directly inside a holder), ADR 3570, ADR 3781; ISO 32000-2 §9.10.3; ISO
19005-2 section 6.2.11.7.2, ISO 19005-4 section 6.2.10.7, both parts' section 5.1.

## Context

Record 2270 left two gaps in `archive::font_maps`. Its pass walked objects stating `/Type /Font`,
so a font written directly inside another object (a page's `/Resources`, a form's) was not
reached, and the output's own check refused it. And a damaged `/ToUnicode` that also states a
placeholder was answered by two routes: `font_maps` wrote the map again at its own object, while
`to_unicode::derive`, where no map could be derived from glyph names, took the placeholders out of
the *producer's* bytes and pointed the font at that new stream — which still held what the reader
read past, so section 5.1 still failed.

## Decision

1. **A font inside a holder is restated in place.** The pass also walks every object for fonts
   found where `pdf_archive`'s population finds them (`sites::fonts_inside_edited`, the walk
   `typed_fonts` uses), restates each as a font of its own is restated, and records the holder;
   the rewrite edits the holder's fonts there, the four map entries alone, and leaves a font the
   substitution writes out as an object of its own.
2. **The placeholders route starts from the map as read.** `to_unicode::restated` takes the
   placeholders out of `font_maps::map_as_read`'s program — the producer's bytes where the map read
   whole, the program written as read where it did not — so each removal is proved against the
   reading and the stream the font names is whole and free of placeholders. Where the conversion
   does not write maps as read (`Rewrite::FontMapsAsRead` not wanted), a damaged map is not
   restated there. A map derived from glyph names replaces the producer's outright, as before.

## Cost

A holder is walked twice, once to restate and once to rewrite; both are bounded by the walk's depth
and run only where the row failed. Both losses — placeholders removed, entries read past dropped —
are reported for such a font.
