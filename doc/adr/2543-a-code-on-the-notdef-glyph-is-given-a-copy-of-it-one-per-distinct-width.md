# 2543 — A code on the `.notdef` glyph is given a copy of it, one per distinct width

Status: implemented. Session 1854 (for a simple TrueType font; a CIDFontType2, a CFF and a Type 1 program are deferred,
section 4).
Context: ISO 19005-2 sections 6.2.11.4.1, 6.2.11.5 and 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1, 6.2.10.5 and 6.2.10.9;
ISO 32000-2 §9.6.5.4; A694 item 3; `doc/pdfa-remedies.md` (R03a, `fonts/no-notdef-glyph-shown`,
`fonts/embedded-programs-define-every-glyph-shown`).
Code: `crates/pdf-transform/src/archive/fonts/notdef_glyph.rs` (new), `archive/fonts/truetype_cmap.rs` (`restate_one`,
`NOTDEF_ROWS`, `notdef_fonts_are_truetype`), `archive/decision.rs` (`Answer::OrStated`, the two rows),
`archive/rewrite.rs` (`Rewrite::NotdefGlyphGiven`), `archive/prepare.rs`, `tests/archive/notdef_glyph.rs`.

## 1. Decision

Where an embedded simple TrueType program leaves a shown code on glyph 0, the conversion appends to the program a copy
of glyph 0 — its outline bytes and its metrics — and restates the program's `cmap` (the Macintosh Roman (1, 0) and
Microsoft Symbol (3, 0) subtables the restatement of ISO 32000-2 §9.6.5.4 already writes) so the code reaches the copy.
The content stream is not touched, the font dictionary's `/Widths` is not touched, and the page draws the marks it drew
before: the copy's outline is the outline this reader drew for the code. Both `fonts/no-notdef-glyph-shown` (section
6.2.11.8, part 4 section 6.2.10.9) and `fonts/embedded-programs-define-every-glyph-shown` (section 6.2.11.4.1) are
answered by it, under the named rewrite `notdef-glyph-given` in the report and in `xmpMM:History`.

A694 item 3 is the authority to bake what the viewer shows into the file when the producer's input cannot be made
correct: the producer's program drew a `.notdef` box, and the archived file draws the same box from a glyph that is no
longer `.notdef`.

## 2. One copy per distinct width, not one per program

ISO 19005-2 section 6.2.11.5 holds the width the font dictionary states for each code and the advance the program
states for the glyph that code reaches to agree within its stated tolerance. The codes on glyph 0 may carry different
`/Widths`, and one glyph has one advance, so a single copy would trade a section 6.2.11.8 failure for a section 6.2.11.5
one. The codes are grouped by their width in the program's units (`units_per_em`), and each group gets one copy whose
`hmtx` advance is that width. The slot's hypothesis — one glyph appended per program — held only for a font whose codes
on `.notdef` share a width; the integration test with two widths gets two glyphs.

## 3. What is grown, what is refused, and the proof

`with_notdef_copied` grows `maxp`, `loca`/`glyf`, `hmtx` with `hhea`, `vmtx` with `vhea`, `hdmx`, `LTSH` and `post`
(formats 1, 2 and 3; a new name is `notdef`, `notdef.1`, … one the table does not hold). It refuses a program with a
bitmap table (`EBLC`, `CBLC`, `bloc`, `sbix`), whose strikes it would have to grow too, more copies than a simple font
has codes (256), copies whose outline bytes together pass 1 MiB (`MOST_COPIED_OUTLINE_BYTES`), an `hdmx` whose header
states more records than the table holds, and a glyph count the 16-bit index cannot reach. Every count is checked
before a table is grown or memory is reserved for it, and a test sits at each bound. Under a long `loca` a copy is the
outline's bytes exactly; under a short one it is padded to the even byte that format needs. The `post` names are one
pass of at most 256 + 65 536 candidates over a set of the names already held. A descriptor the producer wrote inside
the font dictionary is read where it is, and the restated one is written as an object of its own, as ISO 32000-2
§9.6.3 and Table 109 require. After
the rewrite the font is loaded again and held to three things before it replaces the original: every code reaches the
glyph it reached before or the copy given it, by the program's own subtables; each copy's outline is glyph 0's; each
copy's advance agrees with the dictionary's width. A font that fails any of them keeps its refusal, and the refusal
sentence names the font kinds this construction does not reach.

## 4. Deferred

A `CIDFontType2` reaches glyph 0 through `CIDToGIDMap`, so its copy needs that map restated rather than a `cmap`; a CFF
or Type 1 program needs a charstring writer. Each is the same construction over a different table, and is left to the
next archive round in the ranking's order.

## 5. Evidence

Of the ten R03a documents whose fonts are simple TrueType, six convert at 2b and five at part 4 under `as-if-printed`
where HEAD's binaries refused all ten, each output passing veraPDF at its target with as many pages as its source;
the source of one, planted among them, fails veraPDF's section 6.2.11.4.1 test 2. Of the four left, one shows an
operator outside the base standard and a page it cannot photograph, one fails rows of its own beyond R03a, and two —
one stating an `/Encoding` that is a stream, one a two-glyph symbol face — leave the restatement with no code it gave a
copy to, so their rows keep the refusal; why this reader counts no code of theirs on glyph 0 is not yet read. The fifth at part 4 is refused on its document information dictionary. A plant that gave
both copies of the two-width test the first width was refused by the readback; one that moved an advance by one unit
in a 1 000-unit em was not, which is section 6.2.11.5's own tolerance.
