# 3689 — Three readings of §7.3 the reader made in silence are named repairs

Session 2247, the silent-repair campaign's `pdf-syntax` pass over audit 2243. Status: **accepted**.
Context: ADR 3477 (one channel), ADR 3506 (section 5.1 over the log), ADR 3521 (the converter's
answers), ADR 3581 and ADR 3591 (an agreed erratum is the standard); ISO 32000-2 §7.3.5, §7.3.8.1,
§7.3.10; Errata Collection issue #379 (`doc/errata/site/docs/32000-2-2020/PDF-ObjectNumber.ebnf`,
`PDF-GenerationNumber.ebnf`).

## Context

Audit 2243 found three tolerances of the reader that log nothing (B02 to B04), each with a failing
test in `crates/pdf-syntax/tests/objects_7_3_spec_audit.rs`:

- **B02**: `<< /Inner << /Length 3 >> stream … endstream >>` was read as a stream inside the outer
  dictionary. §7.3.8.1: "All streams shall be indirect objects".
- **B03**: `/A#00B` was read as a name holding character code 0, which §7.3.5 excludes ("any
  characters (8-bit values) except null (character code 0)").
- **B04**: `+1 0 R`, `01 0 R` and `1 00 R` were read as references. Issue #379 published a lexical
  grammar for the two numbers — `"1 - 9", [{"0 - 9"}-]` and `"0" | "1 - 9", [{"0 - 9"}-]` — which
  admits no sign and no leading zero; `doc/errata-read.md` recorded the tolerance as owed.

## Decision

Each is read as before — its data is all there, and refusing it would lose a page a reader can
show — and is logged under a class of its own in `pdf_syntax::repair::CLASSES`:

- `syntax/stream-not-indirect` (§7.3.8.1), by the parser where `stream` follows a dictionary that is
  itself an array element or a dictionary's value (the parser's depth is above zero);
- `syntax/name-holds-null` (§7.3.5), by the lexer where a `#` escape expands to 00h. The byte is
  kept: dropping it would make the name another valid name, as `name-escape-incomplete` argues;
- `syntax/object-number-spelling` (§7.3.10), by the parser, for a reference and for an object
  header. The lexer's integer admits a sign and leading zeros, as §7.3.3 lets any integer. The
  lexer marks where the last integer written with either ended — tested only for a token whose
  first byte is a sign, a PERIOD or a `0`, which its dispatch already separates from the digits
  1 to 9 — and the parser compares that mark with the reference's span: one comparison a
  reference. Logging is a cold call out of line. Three earlier versions, each re-reading the
  token's bytes from the parser, put WTPDF's open-instruction band outside (10 409.2 k, 10 290.4 k
  and 10 228.6 k against 10 200.0 k; main 10 094.5 k). `syntax/name-holds-null` is tested in a
  cold function that reads every `#xx` escape: tested in the loop every name runs, it cost 0.2 %
  of the instructions of reading WTPDF's objects (callgrind, 445.70 M against 446.61 M). Out of
  the loop, names cost less than before the check existed, so `PDF20_AN001-BPC.pdf` and
  `bug1815476.pdf` opened below their floors and `doc/checks/launch-path.toml` moved both. Object number 0 is outside the grammar too and is logged by the same class.

`pdf_archive::repairs::MAPPING` maps all three to section 5.1 alone; no part of ISO 19005 states a
rule of its own about any of them. The content grammar's `COVERAGE` lists `name-holds-null` as the
reader's, since the content lexer meets names.

**The converter's answer** is ADR 3521's, unchanged. Outside content, the whole-file rewrite answers
`object-number-spelling` (the serializer writes every number in the grammar). It does not answer the
other two: the serializer writes the nested stream nested (ADR 3690) and the name with its 00h
escaped back as `#00`, because the name's identity is its bytes. So the output's own check logs the
class again, and the refusal names it (`declined_output`). Inside content, `name-holds-null` is
refused by the fence's general sentence (`IN_CONTENT`); a sentence of its own in
`IN_CONTENT_BY_CLASS` was not added because another lane is editing that table this session.

## Consequences

- The three audit tests pass, and their manifest entries are removed.
- Hoisting a nested stream into an indirect object of its own, so that a rewrite answers B02, would
  be a numbering change made before the write. It is left, and it is named in ADR 3690.
