# 2723 — A code Annex D encodes twice is named for its character and reaches a copy of its twin's glyph

Session 1954, a single archive round.
Status: accepted and built, for a face substituted into a simple `TrueType` font whose program is
a `glyf` sfnt. In `pdf-transform`: `archive/fonts/twin_codes.rs` (`renamed`, `split`, the `cmap`
rewrite), `archive/fonts.rs` (`face_widths`' splits, `split_twins`, `SubstitutedFont::twin_codes`),
`archive/fonts/notdef_glyph.rs` (`with_glyph_copied`, of which `with_notdef_copied` is glyph 0's
case), reported in `report.rs` and the file's `xmpMM:History` (`prepare/font_rewrites.rs`).
Tests: `twin_codes::tests` (4), `twin_codes::a_soft_hyphen_narrower_than_the_hyphen_reaches_a_copy_of_its_glyph`,
`…_at_the_hyphens_width_keeps_the_encoding`. Context: ADRs 2246 (the edition that describes a
file), 2543 (`.notdef`'s copies, the copy this generalises), `doc/pdfa-blocking-issues.md` R10.

## The case

`486832` and `499039` show `WinAnsiEncoding`'s codes 45 and 173 in one font and state 333 and 0 for
them. Both codes name `hyphen` (ISO 32000-2 Annex D, Table D.2), reach one glyph, and ISO 19005-2
section 6.2.11.5 then asks that glyph's one advance to equal two numbers: the fence
`TWO_WIDTHS_FOR_ONE_GLYPH`. Restating `/Widths` is forbidden (`doc/pdf-a-conversion-limits.md`
section 4.9).

## The reading

ISO 32000-1 Annex D, note 5: "The hyphen character is also encoded as 255 in WinAnsiEncoding. The
meaning of this duplicate code shall be “soft hyphen,” but it shall be typographically the same as
hyphen." Note 6 says the same, in other words, of the second space code and the nonbreaking space. ISO 32000-2
keeps the `hyphen` name and shows, in its own example, a producer saying the other meaning with
`/Differences [ 160 /nonbreakingspace 173 /softhyphen ]`.

So for a file ISO 32000-1 describes — at part 2 always, at part 4 where the label is below 2.0, as
ADR 2246 decides for `/CIDToGIDMap` — the encoding is restated with the name of the character the
code already meant, `softhyphen` or `nonbreakingspace`, both rows of the Adobe Glyph List as ISO
19005-2 section 6.2.11.6 requires of a non-symbolic `TrueType` font. Only a shown duplicate whose
stated width differs from its twin's is renamed, and never one the producer's `/Differences`
already names. Where the face then gives the character a glyph of its own, the existing route
restates its advance. Where it does not — Liberation Sans gives U+00AD the hyphen's glyph — the
program gains a copy of that glyph, byte for byte, at the end ("typographically the same"), and
every Unicode `cmap` subtable sends the character to it; its advance is the code's width.

**What it costs, stated**: text extracted from code 173 is U+00AD rather than U+002D. That is the
character ISO 32000-1 assigns the code, so for the files this applies to it is a restatement of
what the file meant, reported on the font's line and in the history as an interpretation.

## Not done, and why

`499039` also shows the pair in a `TimesNewRomanPSMT` substituted by Foxit Serif, a CFF face named
by glyph name with no `softhyphen` glyph, and in an embedded subset `TrueType` program on the
restatement route. Each needs its own copy (a CFF charset entry; the embedded program's
`cmap`), so `499039` keeps the fence. A file labelled 2.0 keeps it at part 4.
