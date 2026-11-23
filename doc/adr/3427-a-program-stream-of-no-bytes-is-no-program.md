# 3427 — A program stream of no bytes is no program

Session 2181. Status: **accepted** and built; ADR 3412 item 4. Context: ISO 32000-2 §9.8.1, §9.9's
Table 124; ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1; ADR 2636.

## Reading

`bug866395`'s `/FontFile3 /Type1C` is ten bytes, a valid Flate stream (header 48 89, an empty final
block, Adler-32 1) of **nothing**. veraPDF's "End of file is reached" is the true answer: there is
no CFF header to read. The disagreement was this tree's. `pdf_font::program::no_program` already
reads a stream that decodes to nothing as a descriptor that provides no program and substitutes
(§9.8.1, "when the font program is unavailable"), so `LoadedFont::load` succeeds — and
`fonts/font-programs-embedded` asks for a `Malformed` refusal before it calls a decoded program
unparsed, so it found nothing and the output kept the empty stream.

## Decision

`pdf_archive::program_unparsed` reports a program stream that decodes to no bytes as a program the
file does not hold, with its own reason. The converter's `cut_program` asks the same function, so
the font takes the existing route of a font whose program is not there: the stream is dropped and
the face the window drew in its place is embedded under the descriptor's names and the
dictionary's widths (ADR 2636, ADR 2870).
