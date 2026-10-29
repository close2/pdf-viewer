# 2578 — A code on `.notdef`, or on no glyph, in a simple font's CFF program is given a glyph by writing the program

Status: implemented and measured. Session 1872 (a simple font over a bare CFF, `/FontFile3 /Type1C`; a CID-keyed CFF and a Type 1
program are not built, section 4).
Context: ISO 19005-2 sections 6.2.11.4.1, 6.2.11.4.2, 6.2.11.5 and 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1, 6.2.10.5
and 6.2.10.9; ISO 32000-2 §9.6.5.1 (Table 112), §9.6.5.2 and §9.10.2; Adobe Technical Note #5176 sections 5, 10, 13
and Appendix A, #5177 sections 3.1 and 4.1; A694 item 3, A48; ADRs 2543, 2556 (section 2), 2557.
Code: `crates/pdf-font/src/cff.rs` (`with_glyphs_given`, `GlyphToGive`, `GivenOutline`, `MOST_GLYPHS_GIVEN`,
`read_back`, `name_for`, `string_id`; `with_empty_glyphs` now calls it; `Layout::top_dict` writes a charset entry
where the Top DICT stated none), `crates/pdf-transform/src/archive/fonts/cff_notdef_glyph.rs` (new),
`archive/fonts/truetype_cmap.rs` (`cff_given`, `CmapRestated::cff`, `history`, `NOTDEF_NOT_GIVEN`,
`notdef_fonts_have_truetype_programs`), `archive/rewrite/values.rs`, `tests/archive/cff_notdef_glyph.rs`;
`fuzz/fuzz_targets/cff_write.rs` (new) with its recipe in `fuzz/seeds.sh` and `fuzz/seed_streams.py` and its line in
`doc/verify.md`.

## 1. Decision

Where a simple `Type1` font embeds a bare CFF program and a shown code reaches glyph 0, or no glyph at all, the
conversion appends to the program one glyph for it and leaves the content stream and `/Widths` alone. The glyph draws
what this reader drew (A694 item 3): a code on glyph 0 gets a copy of `.notdef`'s charstring, every byte but the width
operand, read against the same Private DICT and subroutines; a code that reached no glyph drew nothing and gets a
charstring of a width and `endchar`. Each glyph's advance is the width the dictionary states, rounded to the program's
units and held to ISO 19005-2 section 6.2.11.5's tolerance, so codes of two widths take two glyphs (ADR 2543 section 2).
Both `fonts/no-notdef-glyph-shown` and `fonts/embedded-programs-define-every-glyph-shown` answer under the named
rewrite `notdef-glyph-given`, and the file's `xmpMM:History` names the clauses and the fonts.

The slot named only the `.notdef` charstring. The fonts the corpus names carry the second shape in the same font — a
`/Differences` or base-encoding name (`space`, `caron`, `fi`, `uni0020`) the subset program does not hold — and a font
answered for one row and not the other keeps the document refused, so the one writer answers both.

## 2. The name a code keeps

§9.6.5.2 makes `/Differences` name glyph descriptions "that exist in the font program". Where the encoding names a
glyph the program lacks, the new glyph carries **that name**: the font dictionary is untouched and §9.10.2's glyph-name
method still reads the character the producer wrote. Where the encoding names nothing (`WinAnsiEncoding`'s code 10) or
`.notdef` itself, the glyph is named `notdef`, `notdef.1`, … — outside the Adobe Glyph List, so the code gains no
character (A48's condition, as ADR 2458's empty glyph) — and the font's `/Encoding` is restated: a predefined name
becomes the `/BaseEncoding` of a dictionary (Table 112 admits `MacRomanEncoding`, `MacExpertEncoding`,
`WinAnsiEncoding`; any other name is refused), an absent entry becomes a dictionary with no base, which Table 112 reads
against the embedded program's built-in encoding exactly as before, and a dictionary keeps its base with its
`/Differences` re-emitted in runs. A name a glyph already holds is refused rather than given twice. A standard string
is charted under its own SID (TN 5176 Appendix A), a string the String INDEX holds under its existing one. A stated
`/CharSet` gains the new names (ISO 19005-2 section 6.2.11.4.2).

## 3. The writer, its bound and its proof

`with_glyphs_given` grows the `CharStrings` INDEX, restates the charset in format 0 and the String INDEX, and every
Top DICT offset through `Layout::with_glyph_tables`; a Top DICT that stated no charset (the predefined ISOAdobe one,
TN 5176 section 13 — `issue7439.pdf`'s program) is given the entry, without which the fresh charset was unreachable.
At most `MOST_GLYPHS_GIVEN` (256, a simple font's codes) glyphs per call, checked before a table is grown; the shown
codes are a 256-entry table however many strings a page shows; the program bytes count against
`MAX_COPIED_BYTES` before they are copied. **Two proofs**: the rebuilt program is read back by `read-fonts` — the
charstring count, every charstring byte for byte, each new glyph's name through the charset — and the font is then
loaded again from the in-memory objects and held, for all 256 codes, to the outline it drew, its width, the glyph it
reached (a code no page shows may newly reach a given glyph by sharing its name: `WinAnsiEncoding` names `space` at 32
and 160), and for each given code to its glyph with the program's advance within the tolerance. **The plant**: the
`CharStrings` INDEX's last offset written one byte short. The outline proof alone passed it — an empty glyph that has
lost its `endchar` draws nothing either way — and the readback was added for that reason; with it the plant is
refused and no file is written. **The Top DICT's `CharStrings` entry is read once, with one operand** (TN 5176
Table 9): an entry stated twice or with more operands is refused, because `read-fonts` resolves it by its own rule and
the INDEX grown would then not be the one the glyphs were read from — the `cff_write` fuzz target's first finding
(section 5).

## 4. Not built

- **A CID-keyed CFF** (`CIDFontType0C`; four R03a documents, FDSelect formats 0 and 3): a shown CID absent from the
  charset reaches glyph 0 (§9.7.4.2), so the remedy is one glyph per CID (the charset maps one CID per glyph), the
  charset extended with the CID itself, FDSelect re-emitted for the grown glyph count with the new glyphs under glyph
  0's Font DICT, `CIDCount` raised where the CID passes it, the `FDArray` rewritten with shifted Private offsets as
  `Layout::with_charstrings` already does, the copy's width irrelevant to §9.7.4.3 but held to `/W` by section
  6.2.11.5; a shown CID 0 keeps its refusal (ADR 2557).
- **A Type 1 program** (`/FontFile`; four documents): ADR 2556 section 2's list stands — eexec and charstring
  decryption (keys 55665 and 4330), a new `/CharStrings` entry with `hsbw` restated, re-encryption, `/Length1`–`3`
  restated — and the encoding side is this ADR's section 2 unchanged.

## 5. Measured

The census after the change, without and with the CJK pack, against HEAD's: R03a's documents/alone under
`as-if-printed` 23/14 → 14/11 at 2b and at 4; under `as-shown` 24/14 → 21/11 at 2b and 24/13 → 21/10 at 4, the same
with and without the pack. Converted, `as-if-printed`: pdf.js 873 → 875 at 2b and 884 → 886 at 4 with the pack (872 →
874, 883 → 885 without), corpora 405 → 406 and 410 → 411; `as-shown`: pdf.js 812 → 814 and 817 → 819, corpora 337 →
338 and 335 → 336. No line's refused count rose. The three freed are `issue5256.pdf`, `issue7439.pdf` and
`10531_2004_Article_5096977.pdf`; the census's other simple-CFF documents (`027613`, `073439`, `268346`, `477139`,
`507618`, `509284`) lose R03a and stay refused on rows HEAD already refused them on
(`only-operators-the-base-standard-defines`, section 6.1.13's limits, `named-resources-are-defined`).
Of HEAD's 24 R03a documents with the pack, 9 are
simple Type1C, 4 CID-keyed CFF (FDSelect formats 0 and 3), 4 Type 1 and 7 TrueType or `CIDFontType2`: section 4's two
writers are owed four documents each.

veraPDF 1.30.3 passes all twelve outputs (three documents, two profiles, 2b and 4). The planted failure: the three
sources fail it, `issue5256.pdf`'s on 6.2.11.8 among five clauses, so on this program veraPDF does read the `.notdef`
reference that trap 282 found it blind to in a `/Differences` case.

`getrusage` beside HEAD's binary, `archive --to 2b` under `as-if-printed`, a simple font whose 32 shown codes all name
`.notdef` at 32 widths (`ru_maxrss`): a 3-glyph program 12.4 MiB at HEAD (refused) and 13.1 MiB here (written); a
1 980-glyph program whose `CharStrings` INDEX has offset size 2 and last offset 0xFF2B, 13.0 MiB and 14.7 MiB; a
63 549-glyph, 17.5 MB program at offset size 3 and last offset 0xFFFDF0, 45.3 MiB and 185.1 MiB — about ten times the
program, which `MAX_COPIED_BYTES` (32 MiB per conversion) bounds. The writer always writes offset size 4, so neither
boundary is one it can mis-cross, and all three outputs pass veraPDF. 33 shown codes on glyph 0 are refused before the
writer at HEAD and here alike: the checker keeps 32 places (`pdf-archive` `finding.rs`'s `MAX_KEPT`) and a list cut
short names no font (`archive/fonts/truetype_cmap.rs`, `cut_short`).

`cff_write` (`fuzz/fuzz_targets/cff_write.rs`, new), built with `-s none` in its own target and seeded with the
writer's outputs and inputs above (28 programs): its first run found, after 99 s, a `CharStrings` entry of three operands
— the writer grew the INDEX at the first, `read-fonts` read an empty one at the last — and its second, after 81 s, a
`Private` entry of five operands whose restated offsets the writer's own reader refused. Both are refused now (section
3; `a_charstrings_entry_of_three_operands_is_refused` holds both programs, minimised), and the third run ended clean:
3 677 349 executions in 601 s, 1 286 edges, 44 MB resident.
