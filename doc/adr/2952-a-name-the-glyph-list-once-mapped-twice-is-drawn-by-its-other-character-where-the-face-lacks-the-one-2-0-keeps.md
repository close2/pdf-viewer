# 2952 — A name the Adobe Glyph List once mapped twice is drawn by its other character where the face lacks the one 2.0 keeps

Session 2069. Status: **implemented.** Context: A888 (the owner's answer to Q888), ADR 2935 (whose section 2 this
amends), ADRs 2630 and 2934, ISO 32000-2 §9.6.5 and §9.10.2.

## 1. The question

ADR 2935 named the pack's glyphs by the Adobe Glyph List 2.0 alone, so a face without U+2206 INCREMENT had no
`Delta`, and ADR 2630's Greek document, whose unembedded `Times-Roman` shows `/Delta` at code 135, stayed
photographed under `as-if-printed`. Q888 asked whether the builder may draw such a name with the Greek letter's
glyph; the owner answered yes (A888, 2026-11-17).

## 2. This is a choice, not a reading

§9.10.2 sends a reader to the list for a name's character, and the list as it stands maps `Delta` to U+2206 alone.
Nothing in ISO 32000-2 says which glyph a face lacking U+2206 should draw for `Delta`, and INCREMENT and the Greek
capital delta are separate characters with no decomposition between them. Drawing the Greek letter is the owner's
choice for names the list now maps elsewhere, made because the standard 14 metrics name the glyph `Delta` and the
list's own earlier version gave it both characters. It is recorded as that choice (principle 5), not derived.

## 3. The table and where it came from

Version 1.2 of the list (22 Oct 1998) says of itself "12 glyph names are mapped to two UVs each" and marks the
second value of each `Duplicate`. Adobe no longer publishes it; the copy read is the Wayback Machine's of
`partners.adobe.com/asn/developer/type/glyphlist.txt` at 20010215130957, SHA-256
`62ad74724df1b2ea160fe45afd414cc6382e21872235249224468360521b855c`. The twelve
are `Delta`, `Omega`, `Scedilla`, `Tcommaaccent`, `fraction`, `hyphen`, `macron`, `mu`, `periodcentered`,
`scedilla`, `space` and `tcommaaccent`; 2.0 keeps the first value of each. The builder's `DOUBLE_MAPPED` holds
each name with its `Duplicate` value.

## 4. The decision

A name of the table takes the glyph of its `Duplicate` character only where the source face maps no glyph for the
character 2.0 gives it and does map the `Duplicate` one. Where the face maps neither, the face has no glyph of that
name and a font showing it keeps its refusal or photograph, as before. Against the pinned sources this reaches one
name: `Delta`, in the four Noto Serif faces (Noto Sans Mono maps U+2206). Every other name of the twelve finds its
2.0 character in every source, so its glyph is unchanged.

## 5. Consequences

The four serif programs gain one glyph each and their digests move; the pack becomes `quorra-lgc-pack-2.zip`. Two
builds wrote the same bytes. The pack is read only by the archival conversion, so no window page moves.
