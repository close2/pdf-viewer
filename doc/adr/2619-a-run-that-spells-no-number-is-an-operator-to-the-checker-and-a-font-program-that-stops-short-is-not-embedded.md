# 2619 — A run that spells no number is an operator to the checker, and a font program whose decode stops short is not embedded

Status: implemented. Session 1892, the second archive slot of batch 130. The checker's half of what
ADR 2618's gate freed: two documents this tree wrote as conforming and veraPDF rejected.

## Context

ADR 2618 lets `as-if-printed` photograph a page whose own stream is damaged. Two documents it freed,
`509284` and `435321` of the govdocs1 error set, were then written and passed by `pdf-archive`, and
veraPDF 6.2.2 rejected both on pages the photograph never touched. The gate was right; the checker
had passed bytes the converter copies verbatim from the source.

- `509284`, pages 9 and 18: a FlateDecode stream that stops short, whose last decoded bytes are
  `454` followed by a byte above 127; page 14: `3NT-DE290.58` among the path operators. The lexer
  salvages `454` and `3` off the front of each run and records the rest (`pdf_syntax::lexer::Salvage`),
  and the content reader keeps the salvage wherever the tail names no operator (ADR 1004). The
  survey walks through the same reader, so 6.2.2's row saw two numbers.
- `435321`, page 1: Wingdings is a Type 0 font whose descriptor states a FontFile2 stream, and that
  stream's Flate data stops at a back-reference past its window. The row asked only whether a key is
  stated.

## Decision

1. **The survey reads a run as written.** ISO 32000-2 §7.2.3: "A sequence of consecutive regular
   characters comprises a single token." A run §7.3.3 does not spell is no number, so it is a
   keyword, and §7.8.2 makes a keyword in a content stream an operator. Section 6.2.2 of both parts
   asks which operators a stream uses, so `ContentReader::with_runs_as_written` hands the whole run
   over, and the survey's two readers (the page's `/Contents` and every nested stream) use it. The
   drawing reader keeps ADR 1004's salvage: what it draws is unchanged, and the mode is asked only
   of a salvaged token, so a number the clause spells costs nothing.
2. **A program whose decode stops short is not embedded.** §9.9 embeds a program "as data contained
   in a PDF stream object"; a decode that ran out or met what its filter's grammar does not admit
   (`Damage::stops_short`) holds the front of a program, and no byte of the file states the rest.
   `fonts/font-programs-embedded` now fails it as it fails a font that names no program. A check
   value that disagrees over every byte, and a filter this reader does not decode, are passed over:
   neither says the program is absent.

## Consequences

The census against HEAD's, with ADR 2618's gate beside this one: corpora `as-if-printed` converts
411 → 419 at 2b and 416 → 427 at 4 (either part 417/8/59 → 425/12/47), against 422 and 431 with the
gate alone. `509284` converts and passes veraPDF at 2b and 4: the survey now names pages 2, 4, 9,
14 and 18 under 6.2.2, and the gate photographs each. Five documents the gate alone wrote or HEAD
converted now refuse under decision 2, each on a §9.9 program the file holds the front of:
`435321` (FontFile2, Corrupt), `178360` (FontFile3, Truncated and Corrupt), `268346` at 4 (FontFile3,
Truncated), `F001u_3_7j` of PDFBox (FontFile2, Truncated) and pdf.js `issue11651` (FontFile2,
Corrupt); the last two converted at HEAD. `027613` in `as-shown` no longer reads a `.notdef` show
in F182; it stays refused on four other requirements.

This reading is stricter than veraPDF's in one place: `33600.272.58`, two periods, is no §7.3.3
real, and this checker fails it where veraPDF reads a number. The specification decides (principle
5). A remedy that rewrote such a run to the number the reader draws, or re-embedded a program cut
short, would free more pages without a photograph; neither is built. The witnesses list
(`tests/witnesses.rs`) was not extended: both source documents fail these rows for other reasons
too (`435321` embeds no BookAntiqua or Arial), so a source witness discriminates nothing. The
fixtures `a_run_that_opens_with_a_number_and_spells_none_is_an_unlisted_operator` and
`a_program_whose_decode_stops_short_is_not_embedded_and_a_whole_one_is` pin the shapes.
