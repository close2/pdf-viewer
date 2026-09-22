# 1221 — Table 124 is one function, read in the writing direction

Status: accepted and **built**.
Context: `crates/pdf-font/src/embedding.rs` (new), `crates/pdf-font/src/{cff,sfnt,lib}.rs`,
`crates/pdf-transform/src/archive/fonts.rs`, `crates/pdf-transform/tests/archive.rs`.
Answers: `doc/adr/1209` section 3's first remaining build — "Table 124's `OpenType` row, which
admits an sfnt carrying a `CFF ` table under a `Type1` dictionary and needs that table read out of
the wrapper".
Clauses: ISO 32000-2 §9.9 (Tables 124 and 125), §9.9.1, §9.9.2, §9.7.4.2, §9.6.2.2; ISO 19005-2
section 6.2.11.4.1.

## 1. What was wrong: one row of the table was read and three were not

`key_for` stated two pairings — a `glyf` sfnt under a `TrueType` dictionary, a bare CFF under a
`Type1` or `MMType1` one — and answered `None` for everything else. That is not Table 124; it is
two of its rows. The table has four, and the fourth has three bullets of its own, so a conversion
refusing "the face is not in a format Table 124 admits" was often refusing a pairing the table
states outright.

`key_for` is now the whole table, and the match arms read as its rows do. Two of the table's own
distinctions turned out to be load-bearing and are easy to lose on a fast reading:

- **`MMType1` is in the `Type1C` row and not in the `OpenType` one.** The `OpenType` row's third
  bullet says "A Type1 font dictionary or CIDFontType0 CIDFont dictionary", and `MMType1` appears
  nowhere in that row. So a multiple-master dictionary takes a bare CFF and not a wrapped one.
  The corpus has the witness: `6-2-11-4-1-t01-fail-b.pdf`'s `/IYSDLG+OceanSansMM_648_475_` is an
  `MMType1`, and naming a CFF-flavoured OpenType face for it is refused while naming a bare CFF
  converts the document.
- **A `glyf` face has no row under a `Type1` or `MMType1` dictionary at all.** Every key the table
  opens to those two carries Compact Font Format — `/FontFile` a Type 1 program, `/FontFile3`
  `/Type1C` a bare CFF, `/FontFile3` `/OpenType` an sfnt whose `CFF ` table has a Top DICT without
  CIDFont operators. That is the one refusal worth a sentence of its own, because the advice a
  general "Table 124 does not admit this" gives is wrong: naming another file of the same face does
  not help, and naming a CFF-flavoured face does.

## 2. The `OpenType` row turns on the program's own bytes, so the program is asked

The row's conditions are about what the file *contains*: a `glyf` table, or a `CFF ` table with a
Top DICT that uses CIDFont operators, or a `CFF ` table without them — and, for the two `CFF `
bullets, a `cmap` beside it. None of that is in the dictionary, and a producer's `/Subtype` cannot
settle it, so `pdf_font::embedding` reads the sfnt's own table directory and hands the `CFF ` table
to this crate's CFF reader to ask whether its Top DICT uses CIDFont operators.

`SfntTables` answers tags by lookup rather than as named fields. Which tags matter is Table 124's
question and not the reader's: the `FontFile2` row names six, the `OpenType` row's three bullets
name different ones each, and ISO 19005-4 section 6.2.10.5 adds `vmtx`. A reader that enumerated
them would have to be edited whenever a caller read one more line of the table.

**The container is identified by its version tag and `ttcf` is deliberately not one of the three
accepted**, on §9.9.1's "an embedded CFF font file in PDF shall consist of exactly one font": which
face of a collection a descriptor carries is not a question the bytes answer.

## 3. What the advances do, which is nothing

The whole of ADR 1209's proof carries over unchanged, and that is the point of keeping the face on
one path: `pdf_font::restate` makes a supplied program's advances the numbers the dictionary already
states, `proves` reads the program back through the same two readers a caller holding nothing but
bytes has, and the `/Widths` array is the producer's byte for byte. For an sfnt carrying `CFF `
outlines the advances live in `hmtx`, which is where `restate` already writes them and where the
validator already reads them, so neither side needed a second reading.

Table 125's `/Subtype` is `OpenType` for that stream, which is the table's own word: "The name shall
be Type1C for Type 1 compact fonts, CIDFontType0C for Type 0 compact CIDFonts, or OpenType for
OpenType fonts." `/Length1` stays off a `/FontFile3`, as before.

## 4. What it closes, measured

**Nothing in the corpus by itself, and the measurement says why rather than leaving it at that.**
`6-2-11-4-1-t01-fail-a.pdf` is the only corpus `Type1` dictionary without a program, and it already
converted with the shipped Foxit serif face; naming `NimbusRoman-Regular.otf` with `--font` now puts
that face in as `/FontFile3` with `/Subtype /OpenType` and the output is held to PDF/A-2b with every
requirement met. The row is exercised on a real document rather than only on a fixture.

**One document the previous round counted as unreachable was reachable all along**, which is the
correction this ADR owes ADR 1209 section 3. That round measured `--font` with the machine's
`glyf`-based faces and concluded the flag closed none of the thirteen; `6-2-11-4-1-t01-fail-b.pdf`
and its part-4 twin are `MMType1` dictionaries, and naming a *bare CFF* — one of this program's own
Foxit faces — converts both, under the `Type1C` row that was already written. The refusal did not
say so, which is why it now names the glyf-under-Type1 pairing and what resolves it.

The default sweep is unchanged, and has to be: `--font` is never a default, so a corpus run that
supplies nothing reaches none of this.

## 5. Where the fixture came from

`crates/pdf-transform/tests/archive.rs` builds its CFF-flavoured OpenType face around one of this
program's own bare CFF programs. Table 124's own exemption is what makes that a font rather than a
prop — "not all tables are required in the font file, as described for each type of font dictionary
that can include this entry" — so the container carries the `CFF ` and `cmap` the row names plus the
`head`, `hhea`, `hmtx` and `maxp` an sfnt reader needs before it can state an em square, a glyph
count or an advance. Every number in those four is read out of the `CFF ` table itself, so the
wrapper describes the program it wraps and asserts nothing of its own.
