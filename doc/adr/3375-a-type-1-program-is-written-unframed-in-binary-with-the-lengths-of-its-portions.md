# 3375 — A Type 1 program is written unframed, in binary, with the lengths of its three portions

Session 2168. Status: accepted. Context: ISO 32000-2 §9.9 (Tables 124 and 125, NOTE 2); Adobe Type 1
Font Format sections 2.4 and 7.2; Adobe Technical Note 5040 (PFB); ISO 19005-2 section 6.2.11.4.1,
ISO 19005-4 section 6.2.10.4.1; ADR 2307 (`fonts/font-programs-filed-by-their-format`); round 2164's
veraPDF triage.

## The question

veraPDF could not parse three `/FontFile` programs this tree carried as conforming:
`bug860632` and `issue14462_reduced` keep PFB segment headers in the stream, and `issue4630` states
`/Length2 0` over an encrypted portion written in ASCII hexadecimal. Table 124 files under
`/FontFile` the program "in the original (noncompact) format described in Adobe Type 1 Font Format";
Table 125's three lengths divide it into its clear-text, encrypted and fixed-content portions; and
§9.9's NOTE 2 says "PDF supports only the binary format" for the encrypted one. Our validator
judged none of this: a PFB program named no format, so the row skipped it, and the lengths were
read nowhere.

## Decision

1. `pdf_archive::type1_sections` measures a program: PFB headers removed (tolerating a last segment
   that overstates its length and bytes after a segment that open no header — both corpus shapes),
   the clear text ending after `eexec` and its white space, the fixed content starting at the first
   of the zeros before the last `cleartomark` (0 where absent, which Table 125 allows), and a
   hexadecimal encrypted portion (section 7.2's test: the first four bytes all hexadecimal digits)
   held as the binary it encodes.
2. `fonts/font-programs-filed-by-their-format` reads a framed program's format unframed, and for a
   program filed under `/FontFile` reports PFB framing, a hexadecimal encrypted portion, or a
   `/Length1`/`/Length2` that does not divide the program where its portions meet. A boundary
   stated anywhere in the white space at a portion's end is accepted: producers count that line
   ending on either side, and no reader is misled by it. Where the program holds no fixed-content portion,
   nothing marks where the encrypted one ends, so any `/Length2` past its first byte is accepted:
   `bug1157493` states a `/Length3` of 532 over a program with no zeros at all, and veraPDF reads it.
3. The row's remedy (`FontProgramRefiled`, `font_files::refiled`) writes such a program whole:
   the measured program and the three measured lengths under `FlateDecode`. Every byte of the
   program is the producer's or the binary its hexadecimal digits spell, and this reader already
   read it so, so no glyph changes and no loss is named.

## Costs

The fixed-content boundary is found by walking back from `cleartomark` over zeros and white space,
so an encrypted portion whose own last bytes are `0` or white space is cut a byte or two early in
the measurement; the stated-length check is lenient there and a rewrite would move those bytes into
the fixed-content portion, past `closefile`, where nothing reads them.

## Consequences

`bug860632` and `issue4630` pass veraPDF at 2b and 4. `issue14462_reduced` is unframed and measured,
and veraPDF still fails it inside the decrypted Private dictionary ("expected TT_NAME, but got
TT_INTEGER"): a defect of the program's content, which this rewrite does not touch.
