# 483 — The band has a shape, and the annex prints the character

**Finding.** ADR 0311's 1342 unnamed codes were one number, and the round was asked for their
**shape** and then to close whatever part of it the standard reaches. `pdf_font::NamingGap` is the
instrument — for a code no method named, *which of §9.10.2's methods was the highest-priority one
that font could have answered with*, because the clause ranks its own methods and reporting the
last thing tried would describe every gap identically. Derived by running `LoadedFont::text` and
classifying its failure, so the census cannot drift from the extraction it measures;
`Interpretation::codes_without_a_character` is now a counter per cause with `total()` equal to what
it was, and `examples/unnamed_code_census` is the command. **The shape is not the hypothesis**:
761 codes are an `Identity` ordering with no `/ToUnicode`, 247 a glyph name neither published list
holds, 210 a producer's `/ToUnicode` omitting a code it shows, 123 a code selecting a glyph by code
that nothing names, 1 a registered collection with nothing for the CID — and reading them meant
opening the files, which is where `G14`, `LW010000`, `c128` and `circlecopyrt` (one code each in
fourteen documents) turned up. **The one population the standard answers is `ZapfDingbats`, and
the answer is Annex D.6's third column.** §9.10.2's second method cannot help — the Adobe Glyph
List does not hold `a1`, and Annex D.1 says why: "[t]he characters for ZapfDingbats are ordered by
code instead of by name, since the names in that font are meaningless". But Table D.6 prints
`CHAR`, `NAME` and `CODE`, D.1 opens "[t]his annex lists the character sets and encodings that
shall be predefined in any PDF processor", and §9.6.5.1 names the two symbolic standard-14 fonts as
the ones "whose encodings and character sets are documented in Annex D" — so a processor
predefining this set has predefined what its codes represent. **The `CHAR` column is the standard's
own text and not a picture**: `pdffonts` over the two pages carrying the table lists Cambria,
Calibri, Arial and MS-Gothic and no `ZapfDingbats`. Keyed by code within *that font's own*
encoding, never by name, which is what leaves ADR 0311's refusal standing —
`french_diacritics.pdf`'s Type 3 `/a192` is still counted, and the corpus says so rather than the
argument. **114 codes closed, and a defect no count could see**: a dingbat at code 0x21 was not
unnamed, it read back as `!`, because §9.10.2's printable-ASCII permission — justified as "the
range in which a byte and a Unicode code point mean the same character under every encoding §9.6.5
states" — was being applied to a font whose encoding is not Latin. What is left is argued as the
clause's own answer rather than as a backlog.

**Date.** 2026-08-13.
**ADR.** [0318](../adr/0318-the-band-has-a-shape-and-the-annex-prints-the-character.md).
**Touched.** `crates/pdf-font/src/lib.rs` (`NamingGap`, `LoadedFont::naming_gap`,
`selected_glyph_name`, the `symbolic_set` field and its one reader in `text`, `symbolic_set()`),
`crates/pdf-font/src/encoding.rs` (`ZAPF_DINGBATS_CHARACTERS`, `SymbolicEncoding::character_for`,
three tests), `crates/pdf-model/src/content.rs` (`UnnamedCodes` and the field it replaces, the
tally in `show_text`, `Font::naming_gap`, `PDFVIEWER_TRACE_UNNAMED_CODE`),
`crates/pdf-model/src/type3.rs` (`Type3Font::naming_gap`),
`crates/pdf-model/examples/unnamed_code_census.rs` (new),
`crates/pdf-model/examples/readback.rs`, `crates/pdf-model/tests/{silent_fonts,accessibility,corpus}.rs`,
`doc/conformance/ledger.toml` (§9.10.2, D.6), `doc/todo/21` (§5), `doc/adr/0318-*`, this file.

## The numbers

Every gate ran after the last edit:

| gate | before (session 476) | after |
|---|---|---|
| pdf.js text | 99.3% (24010/24189 words), 22 below floor, 62 not gated | identical |
| PDFBox frozen text | 99.8% (14257/14281) in both orders, 4 below floor | identical |
| corpus | 65 incomplete; 5 codes with no glyph over 2; 57 blank over 9; **1342 unnamed over 45** | 65; 5 over 2; 57 over 9; **1228 over 43** |
| oracle | 906 agrees, 67 contradicted, 786 ambiguous, 19 no render | identical |
| quorra | 918 agree, 37 differ, 1 refused, 18 not comparable | identical |

`dates`, `xmp` and `jpeg2000` pass. `fmt`, `clippy --workspace --all-targets`,
`nextest run --workspace` (1727 passed), the doctests and `cargo test -p conformance` are clean,
the last re-run after the ledger edit.

**Both text gates are flat and the reason is concrete rather than "they strip whitespace".** The
two documents this closed are outside what either gate can see: `reference_words` trims every
token to its alphanumeric core and drops anything under three characters, so a `pdftotext` line
holding one dingbat is not a word — and `pdftotext` reads those characters correctly, which is
evidence about the reading and not its source. `issue15716.pdf` it extracts nothing at all from.
A gate that cannot see a population is not a gate that says the population is fine, which is why
the census exists.

The census is the round's own instrument:

```sh
cargo run --profile gates -p pdf-model --example unnamed_code_census -- doc/pdf.js/test/pdfs/*.pdf
```

## What the round did not do

- **No report**, on ADR 0152's unchanged arithmetic: 43 documents that mostly draw perfectly would
  cost the oracle 43 judged pages for a shortfall in the readback.
- **No `zapfdingbats.txt`.** Adobe's list would have produced the same table from the wrong source
  — and would have offered no principle for excluding pdfTeX's `/a192`, which the annex's own key
  supplies.
- **No Annex D.6 route for an embedded font using those names.** §9.6.5.1 gives an embedded program
  its own built-in encoding; the annex documents the `ZapfDingbats` font program's.
- **No widening of `text_from_the_code`.** Its bound is untouched for every Latin encoding; what
  changed is that a font whose character set the standard documents now answers before that
  permission is reached.

## The two things worth carrying forward

- **A count that stops falling is an answer.** Three of the four large populations are the clause's
  own "there is no way", and saying so with the split in front of it is worth more than a plan to
  close them.
- **A silence in this project decays, and this one had been standing since Annex D.6 was first
  transcribed.** The names were taken out of that table and the characters beside them were left on
  the page — which is `CLAUDE.md`'s own warning about a claim that the specification defines
  nothing, one column to the left of where it usually appears.
