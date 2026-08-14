# Session 506 — the token that borrowed its bytes, and the number parsed in place

Date: 2026-08-14.

Took `doc/todo/44` §2's lexer candidate, and then its number-parsing candidate, because the
first landed cleanly with round left — ADR 0341 has the argument, the caller survey, and every
number.

- `pdf_syntax::Token` became `Token<'a>`: `Keyword` borrows its bytes from the input,
  `Name`/`String` stay owned because they are decoded forms. `read_regular_run` stopped
  ending in `.to_vec()`, and `read_number` stopped copying the run into a `String` before
  parsing it.
- `fixed_format_number` parses exactly the grammar §7.3.3 states — sign, digits, at most one
  period — with an exactness argument (mantissa and power both exactly representable, one
  correctly rounded division) that makes it bit-identical to `f64::from_str` on everything it
  accepts, asserted by a differential test over every digit string to five digits; everything
  else falls back to the standing path unchanged.
- Measured with ADR 0332's and ADR 0330's instruments, three arms from one tree, pool pinned:
  the owner's document 22 397 708 918 → 13 487 421 385 instructions (−39.8%), the
  corpus-normal cold sweep 37 180 286 243 → 34 961 497 466 (−5.97%), readback byte-identical
  on both documents on all three arms.
- Gates: fmt clean, clippy silent, nextest workspace all passed, doctests, conformance
  (7662 citations, 752 quotations verbatim), corpus (62 incomplete, unchanged), oracle
  (ratchets held, 99.8% cache hit rate), text_extraction (both gates), `lexer` and `object`
  fuzz targets 50 000 runs each, clean.
