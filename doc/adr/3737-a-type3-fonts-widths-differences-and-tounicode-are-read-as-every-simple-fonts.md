# 3737 — A Type 3 font's widths, differences and `/ToUnicode` are read as every simple font's

Session 2259, the silent-repair campaign's census burn-down, sixth pass. Status: **accepted**.
Context: ADR 3477, ADR 3565, ADR 3574, ADR 3710; ISO 32000-2 §9.6.4, §9.6.5.1, §9.10.3, Table 110,
Table 112.

## Context

`pdf_model::type3` reads a Type 3 font dictionary itself, beside `pdf_font`'s reader of every other
simple font, and it read in silence what `pdf_font` reads and logs: a `/FirstChar` that is no
integer as 0, an absent `/LastChar` as the array's own length, an absent `/Widths` as no widths, an
element that is no number as 0, and a `/Differences` code outside a byte or a name before any code
skipped. Table 110 makes `/FirstChar`, `/LastChar` and `/Widths` required, `/Widths` "\[a]n array
of ( LastChar - FirstChar + 1) numbers". Its `/ToUnicode` was parsed from the stream's bytes alone,
so a map built on another by `/UseCMap` lost the base `pdf_font` follows, and an entry that was not
a stream was read as absent without a word.

Reading the module's text against §9.6.4 also found a defect: `text` looked a code past one byte up
as code 0, so code 256 took code 0's glyph name's text.

## Decision

- **`widths`** reads Table 110's three entries in one place and records `font/widths-malformed`
  once where any is not as the table states, an array short of its range included; what is read is
  `pdf_font`'s reading, the missing code given Table 110's width outside the range, 0. The class is
  `pdf_font`'s own: the departure is the same row of the same family of tables, and a validator
  maps one name.
- **`/Differences`** records `font/differences-malformed` for a code outside a byte, a name before
  any code, an element of another type, and an entry that is no array.
- **`/ToUnicode`** is read by `pdf_font::to_unicode`, now public: one reader follows `/UseCMap` and
  logs `to-unicode-not-a-stream`, `to-unicode-damaged` and `to-unicode-past-a-bound` for every font.
- **A code beyond one byte** names no glyph and so no name (§9.6.4); a test pins 256.
- The repairs are placed at the entry where the file writes it as an indirect object, else the whole
  file: the reader is handed the dictionary and not its object, as `pdf_font`'s are.

## Consequences

- `HELD`: `type3.rs` 7 → 0. No new class, no mapping row, no converter answer: both classes were
  mapped and answered (ADR 3521: the whole-file rewrite) before this.
- `pdf_font::to_unicode` is a public function (the fuzz workspace was checked).
- Nothing is added per glyph: the checks run once per font read.
