# 2948 — An empty encoding slot names `.notdef`, and advances by it

Session 2067. Status: **accepted and built.** In `pdf-font`: `metrics::notdef_advanced` and
`LoadedFont::advanced_by_notdef`. Context: ADRs 2942 (a name the font lacks), 2687 (a standard font stating a
descriptor), 0932 (an embedded Type 1 program's unassigned codes), 0254 (Errata Collection 3). Clauses: ISO 32000-2
§9.2.4, §9.6.5.1 (Table 112), §9.6.5.2, Annex D (Table D.1); the Adobe Type 1 Font Format, Version 1.1, section 2
(a normative reference, clause 2).

## Context

ADR 2942 advanced a code naming a glyph the drawn face lacks, in one of the fourteen stated by name alone, by that
face's `.notdef`, and left out a code its encoding names nothing for: `StandardEncoding`'s codes below 32, or a
`MacRomanEncoding` slot no character occupies. Such a code reached no glyph and took Table 120's default of 0.

## Decision

**An empty slot names `.notdef`, so it is ADR 2942's case, and the width is that glyph's.** The chain, each link
the text's:

1. §9.6.5.1: "Except for Type 3 fonts, every font program shall have a built-in encoding." §9.6.5.2: the built-in
   encoding "shall be defined by an 'encoding' array that is part of the font program", and "An Encoding entry in the
   PDF font dictionary, if present, shall override a Type 1 font's mapping from character codes to character names"
   — it replaces the names, not the mechanism. Table D.1 calls `StandardEncoding` "the built-in encoding defined in
   Type 1 Latin-text font programs".
2. The Adobe Type 1 Font Format, which clause 2 makes normative, defines that array: BuildChar "begins by using the
   character code as an index into the Encoding array in the font dictionary to obtain the name of the character to
   be built", for every code, and its program example builds the array as `0 1 255 {1 index exch /.notdef put } for`
   before the assignments. A slot no character was assigned to therefore holds the name `.notdef`.
3. §9.6.5.2: "All Type 1 font programs shall contain an actual glyph named .notdef", so the code paints that glyph,
   and §9.2.4 moves the text position by "the distance the current text position shall move (by translating text
   space) when the glyph is painted". ADR 2942's argument against Table 109's `/MissingWidth` applies unchanged: a
   dictionary stated by name alone has neither range nor descriptor.

So the code is advanced exactly as a `/Differences` entry naming `/.notdef` outright is (250 in the Times faces, 600
in Courier). A dictionary stating a descriptor keeps ADR 2687's `/MissingWidth`. The drawing is unchanged: the shipped
faces' `.notdef` paints nothing.

The example in step 2 is an example, not a sentence with a modal verb; the normative part is that every code indexes a
256-element array of names. What a slot holds when nothing was assigned is the example's construction, which is also
PostScript's (an array element is never absent). Recorded as read, not as a silence.

## Consequences

A name-alone standard font showing a control code (a stray line feed in a string, say) now leaves a `.notdef`'s gap
where it collapsed to nothing. No `raster_golden` page is known to show one; a page that moves would be one whose text
under a name-alone font of the fourteen carries a code `StandardEncoding`, `MacRomanEncoding` or Symbol's or
ZapfDingbats' own encoding leaves empty. **Not changed, and reported**: ADR 0932 draws nothing for a code an embedded
Type 1 program's array never mentions, reading it as unencoded; step 2 reads such a slot as `.notdef`. Its width is
the file's `/Widths` there, so only the glyph differs, and only where the designer's `.notdef` paints; it is left to
its own round.
