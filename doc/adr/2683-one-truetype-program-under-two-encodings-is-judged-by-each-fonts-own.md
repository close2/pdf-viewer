# 2683 — One TrueType program under two encodings is judged by each font's own

Session 1931, a single archive round.
Status: accepted. No converter change; the checker's reading is pinned by
`one_program_under_two_encodings_is_judged_by_each_fonts_own_encoding` in
`crates/pdf-archive/src/table/fonts.rs`.
Context: round 1929's census, where veraPDF 1.30.3 failed `AIAA-2005-6173-712` under both stating
profiles at 4 (ISO 19005-4 section 6.2.10.5, test 1) and at 2b (ISO 19005-2 section 6.2.11.5,
test 1); A691/A694 (the standard governs over a validator's defect).
Clauses: ISO 32000-2 §9.6.5.4, §9.6.2; ISO 19005-2 section 6.2.11.5, ISO 19005-4 section 6.2.10.5.

## What veraPDF measures

Eight uses of code 45 in the font `Helvetica` (resource `R10`), for example object 324 of the
part 4 output. The font is a nonsymbolic TrueType font (`/Flags 32`) whose encoding is a direct
dictionary, `/BaseEncoding /WinAnsiEncoding` with `/Differences [... 45 /minus ...]`, and whose
`/Widths` give code 45 the width 584. The embedded program is the substituted Liberation Sans, and
twelve font dictionaries share it (`/FontFile2 314 0 R`). One of them, object 213 (resource
`H11105`), names `/WinAnsiEncoding` and gives code 45 the width 333. veraPDF reports the program's
width as 333.0078, which is `hyphen`.

## Which reading the standard gives

§9.6.5.4: for a nonsymbolic font, a code is mapped to a glyph name through the font's own
`Encoding` entry, its `Differences` applied, then to Unicode through the Adobe Glyph List, then to a
glyph through the (3, 1) subtable. For object 324, code 45 is `minus`, U+2212, glyph 571, 1196
units of 2048, or 583.98. The dictionary's 584 agrees within the clause's tolerance. For object 213,
code 45 is `hyphen`, U+002D, glyph 16, 333.01, and its 333 agrees too. Both fonts conform, and so
does the output.

## Why veraPDF disagrees

veraPDF caches one parsed font program per key, built by `FontProgramIDGenerator
.getTrueTypeFontProgramID` from the stream's object key, the symbolic flag, and the `Encoding`
value's object key, which is the string `direct` for any direct value. Objects 213 and 324 both have
a direct `Encoding` (a name and a dictionary), so they share one cache entry and one code-to-name
table, whichever font was read first. Measured: the part 4 output was edited so that object 213's
`/Encoding` is an indirect object holding the same name, `/WinAnsiEncoding`. The meaning is
unchanged by §7.3.10. veraPDF's 6.2.10.5 failure then disappeared, and nothing else changed except
the length failures that rewriting the file with `qpdf --qdf` introduces.

## Decision

The output is right and veraPDF is wrong, so nothing in the converter changes. Writing each
substituted font's encoding as an indirect object would avoid the validator's cache. It would also
make a choice in our output that answers a validator defect rather than the standard. Under
A691/A694 that is not a reason to change the output. The upstream report is drafted for the owner
in the round's report (`veraPDF-parser`, `FontProgramIDGenerator`). The checker already loads each
font dictionary with its own encoding. The test above pins that: one shared program, one font per
encoding, each judged by its own, and a dictionary that states the other font's width is named.
