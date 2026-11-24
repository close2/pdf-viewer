# 3534 — What the survey reads past a limit in `040669` and `427669`

Status: **accepted** (a reading; the survey is not changed, one comment corrected). Session 2208.
Answers ADR 3519's open point. Context: ISO 32000-2 §7.2.3, §7.3.3, §7.3.4.2, §7.7.3.3; ISO 19005-2
section 6.1.13 (paraphrased); ADR 0733 (the lexer's exponent and its bound).

## Context

ADR 3519's scan decoded every stream with `qpdf` and lexed each alone, and found in these two files
no value past a limit; `pdf_archive`'s survey fails both on section 6.1.13's content-stream row.

## Decision

- **`040669`, page 25 (object 95).** The stream holds runs in §7.3.3's exponential form, which the
  clause forbids a writer: `238.e285`, `62e545`, `147e7465`. `pdf_syntax`'s lexer reads that form as
  the value it glosses (ADR 0733), so `238.e285` is 2.38×10^287 and the other two, past a double,
  are held at `f64::MAX`. The survey records the largest, so the report prints
  `1797…` — the lexer's bound, not a figure the file wrote. The finding stands: every one of the
  three is past the real bound as this reader reads it, and the scan missed them because it
  counted such a run as no number. `note_real`'s comment said an infinity could arrive; none can,
  and the comment now says what arrives instead.
- **`427669`, page 18 (objects 213, 211, 209, …, 199).** Object 209, whose Flate data fails its
  check, decodes to a front ending `(2`: a string opened by its last two bytes. Object 207 begins
  `\r(Frequency, c/yr)Tj`, whose parentheses balance (§7.3.4.2), so the string is still open there.
  §7.7.3.3 reads a page's `/Contents` array as one stream, so it runs through the five streams after
  209 to the end of the page: 62170 bytes, past section 6.1.13's 32767. The scan lexed each stream
  alone and stopped the string at 209's end. The survey's reading is §7.7.3.3's; the file is damaged, and a seam inside a token is the producer's breach of that
  clause: "The division between streams may occur only at the boundaries between lexical tokens".

Neither is a misreading, so nothing in the survey is rebuilt. Both pages are photographed under
`as-if-printed` for the operator row, and with ADR 3535 the photographs answer section 6.1.13 too:
both convert at 2b.
