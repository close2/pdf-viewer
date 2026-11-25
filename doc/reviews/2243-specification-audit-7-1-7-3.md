# Specification audit of ISO 32000-2 §7.1 to §7.3 (syntax overview, lexical conventions, objects) — round 2243

The project's eighteenth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2239 ([`2239-specification-audit-12-1-12-2-12-9-12-11.md`](2239-specification-audit-12-1-12-2-12-9-12-11.md)).
Baseline: main at `6267ba7e`. Evidence:
[`2243-specification-audit-7-1-7-3/`](2243-specification-audit-7-1-7-3/) — `findings.json`
(2038's schema) and `expected-failures.json` (the failing tests of each behaviour finding). The same
entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §7.1 to §7.3 (`doc/md/ISO_32000-2_sponsored_EC3.md` lines 1155
   to 1579: §7.2.2's representation, §7.2.3's three classes and EOL, §7.2.4's comments, §7.3.2 to
   §7.3.10 with Tables 3 to 5) and Annex C's Table C.1 as far as §7.3 cites it were read and set
   against every route that reads or writes the syntax: `pdf-syntax`'s `lexer.rs` (which
   `pdf-model`'s `content/reader.rs` drives for content streams), `parser.rs`, `document/objects.rs`
   (resolution, generations, §7.3.10's undefined reference) and `document/decoding.rs` (Table 5's
   `/Filter` and `/DecodeParms`); the writers `write.rs` (`object`, `real`, `string`, the
   incremental update), `serialize.rs`, `linearize.rs`, `object.rs`'s `Name::escaped`; and the
   content and `/DA` writers in `pdf-model` (`variable_text.rs`, `appearance.rs`) and `pdf-transform`
   (`redact.rs`, `redact/`, `archive/`), the last two by a census of every site that formats a
   number, a name or a literal string itself.
2. 2038's findings on these clauses were read first and are not repeated (T18, D08, B21, T61, D34,
   B45, B54, T74 to T88, D47, T103, T141, T154, D66), nor 2142's B01, B02, B06 and D01 on Table 5.
   Errata: `doc/errata/issues.jsonl` #143, #157, #182, #194, #199, #209, #220, #273, #300, #319,
   #327, #379, #438, #494, #550, #572, #731 and #759, and the published grammars
   `PDF-Integer.ebnf`, `PDF-Real.ebnf`, `PDF-ObjectNumber.ebnf` and `PDF-GenerationNumber.ebnf`.
   #379's grammar settles B04; #438 (duplicate keys compared after escaping) is what the parser
   does; #199 (`/Foo null /Foo null`) and #157 (nulls in arrays) are open and settle nothing here.
3. The lens was *every route*: a requirement on how a token is written binds each writer that
   formats one itself, not only `write::object`; and *a repair is a finding only where it is
   silent* (ADR 3477) — so each tolerance the reader shows was checked for its repair class.

What holds, probed by reading, is not listed one by one. Among it: Table 1's six white-space bytes
and Table 2's eight general delimiters (the braces regular outside a type 4 program, #365); a
comment ending before CR, LF or CR LF; every Table 3 escape, one to three octal digits with the
high-order overflow ignored, a backslash before CR, LF or CR LF dropped with the marker, and an
unescaped CR LF inside a literal string read as one LF; a hexadecimal string's white space ignored
and an odd final digit taken as 0, a stray byte logged; a name's `#xx` expanded before identity,
a malformed escape kept and logged; duplicate keys compared after expansion, the first kept and
logged; a direct null value dropped at the dictionary's reading; an undefined reference, one at
another generation and a chain past the bound read as null; a stream keyword's CR alone, padding and
a wrong direct `/Length` recovered and logged; an exponent and an integer past 64 bits read and
logged. On the way out: `write::real` never writes an exponent and writes the shortest decimal that
reads back; `Name::escaped` escapes `#`, every byte outside 21h–7Eh and every non-regular byte;
strings are written in hexadecimal; `serialize` and `linearize` re-derive each stream's `/Length`
and write every object at generation 0; `redact.rs`'s and `variable_text.rs`'s literal strings
escape both parentheses, the backslash and every unprintable byte in octal.

Readings left, each a question rather than a defect found: `write::real` writes an integral value
of 10^15 or more as a digit run, which this lexer reads past 64 bits as a logged repair; §7.3.3
lets an integer stand for a real, so the file is right and the reader's class is the question.
`pdf-transform/src/archive/languages.rs`'s `literal` escapes the three Table 3 delimiters but not
a CR, which §7.3.4.2 reads back as LF; its one input is a language tag, which holds none.

## Findings

Six behaviour gaps — two in what the writers emit, two in what a writer re-states from a document,
and three tolerances of the reader that log nothing:

- **2243-B01** — `write::incremental_update` writes a replacement stream's dictionary as handed, so
  a `/Length` that does not count the bytes written goes out beside them: a stream read with
  `/Length 3` over 15 bytes is appended as `/Length 3` over the 15. `serialize` and `linearize`
  re-derive the entry. The insert-pages carry (`pdf-transform/src/update.rs`) hands over an
  incoming document's dictionary beside data already decrypted, so an AES source's ciphertext
  `/Length` reaches the same path.
- **2243-B02** — a stream written as a direct value (`<< /Inner << … >> stream … endstream >>`) is
  read as a stream inside the outer dictionary with no repair logged; `write::object` and
  `serialize` then write it back nested, its `/Length` not re-derived.
- **2243-B03** — `/A#00B` puts character code 0 into a name, which §7.3.5 excludes, and the lexer
  logs nothing.
- **2243-B04** — `+1 0 R`, `01 0 R` and `1 00 R` are read as references with nothing logged, against
  the object- and generation-number grammar issue #379 published; `doc/errata-read.md` records the
  tolerance as owed.
- **2243-B05** — a script's `textSize` re-states the producer's `/DA` font in a new `Tf` with the
  name written raw after a lossy UTF-8 decode: `/Helv#20Bold` comes back as `/Helv Bold 14 Tf`.
- **2243-B06** — the `/DA` replay narrows every real operand to `f32`, so a real past its range is
  written into the appearance stream as `inf`, an operator token rather than a §7.3.3 number.

Each has a failing `*_spec_audit` test whose control passes on main:
`pdf-syntax/tests/objects_7_3_spec_audit.rs` (B01 to B04) and
`pdf-model/tests/default_appearance_syntax_7_3_spec_audit.rs`, a module of `forms_suite` (B05,
B06).

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `7.3.8.1` (B01, B02),
`7.3.8.2` (B01), `7.3.5` (B03, B05), `7.3.6` and `7.3.7` (B02), `7.3.10` (B04), `7.3.3` (B06), and
`12.7.4.3` (B05, B06: the `/DA` the clause has a writer build).

## Suggested fix groups

1. **`pdf-syntax/src/write.rs`'s incremental update** — B01: re-derive `/Length` from the bytes for
   every stream it writes and count the disagreements, as `serialize` does; one small round.
2. **`pdf-syntax/src/parser.rs` and `lexer.rs`, the silent tolerances** — B02, B03, B04: a named
   `RepairClass` for a nested stream, for a name escape decoding to 00h and for an object identifier
   outside #379's grammar (the lexer keeps the spelling the parser needs), and B02's writer half in
   `write::object` and `serialize`. Belongs with the silent-repair campaign's `pdf-syntax` pass.
3. **`pdf-model`'s `/DA` writers** — B05 and B06: `appearance.rs`'s `scripted_appearance` writes
   the restated font through `Name::escaped`; `variable_text.rs`'s replay writes reals through
   `pdf_syntax::write`'s formatter. One small round.
