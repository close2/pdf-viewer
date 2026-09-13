# 1015 — The language a character collection does not imply, and two entries nobody asked

Date: 2026-09-13. No ADR: four readings of four entries, none of which a later round must
re-litigate. Files: `pdf-font`'s `substitute.rs` and `substituted.rs`, `pdf-model`'s
`appearance.rs`, `measurement.rs` and `tests/annotations.rs`, `doc/conformance/ledger.toml`.

## §9.8.3 and §9.8.3.1 — Table 122's `/Lang`
The cell says what the entry is for: a name "specifying the language of the font, which may be used
for encodings where the language is not implied by the encoding itself".
`substituted::script_sample` had a character per registered character collection and `&[]` for an
`Identity` ordering, which sent a non-embedded composite font to substitution with nothing asked of
the face. `/Lang` answers there now, with the same three characters and the collection still
believed first. §14.9.2.2 decides the reading: an empty identifier is the file saying the language
is unknown, and "all language tags shall be treated as case-insensitive".

**No corpus document exercises it, and a census says so rather than a guess.** Four files on this
disk state `/Lang` in a font descriptor — `issue15053.pdf` `/ja`, `PDFJS-9279-reduced.pdf` `/EN`,
`ICC.1-2022-05.pdf` `/ja`, `hayro-tests`' `pdftc_900k_0319_page_1.pdf` `/zh-TW` — all four
embedding their program, so none reaches substitution. Trap 8 stated: implemented from the clause,
with the four witnesses calibrating the *reader* (a name in every case, Table 122's own type; the
string arm is §14.9.2.2's). Both rows stay `partial`, now for `/FD` alone — Table 122 says its
values "shall override the corresponding values in the main font descriptor dictionary for that
class of glyphs" and nothing consumes them. §9.8.3's warning is that a parent may be less current
than its child; this time it moved with it.

## §12.5.6.7 and §12.5.6.9 — `/IT` and `/Measure`
Neither entry states a mark, and that is the whole of what is owed. `/IT`'s five values across the
two tables each say what the annotation is *intended to function as*; `appearance::intent` carries
the name, including one its own table does not define, and reports nothing for it — a report names
what this program owes. Which table applies is the `/Subtype`, and that is what the test
discriminates: the key is spelled the same way in four of §12.5.6's subtype tables.

`/Measure` is the annotation's own measuring system, so `measurement::annotation_measurement`
measures its geometry against it rather than against the page's viewport. Table 267 states the
arithmetic and, twice, its order — the conversion before the distance function and before the area
function — and the fixture's `/X` factor of 2 discriminates that order. A polygon gets both `/D`'s
perimeter and `/A`'s area, because Table 267 has an array for each and the clause does not say
which a dimension is; a `/Path`'s curves are not measured. §12.5.6.7 goes `implemented`, §12.5.6.9
stays `partial` for the cloudy `/BE` it always owed.
