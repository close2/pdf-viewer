# 3506 — Section 5.1 is checked over the reader's repair log, and every class maps to its rows

Session 2201. Status: **accepted**. Context: ADR 3477, ADR 3505; ISO 19005-2 and ISO 19005-4
section 5.1, sections 6.1.3, 6.1.7.1, 6.1.9 and 6.1.13; review 2194's findings B01 to B04, B06.

## Context

`conformance/adheres-to-the-base-standard` was `Check::Unchecked`, and a verdict counts only failed
rows, so a file the reader had to repair was judged `Conforms` wherever the specific row did not
re-read the bytes. With every `pdf-syntax` repair on one channel (ADR 3505), the row has an input.

## Decision

- **Section 5.1's row is `Check::Implemented` over the log**: every logged repair whose class is
  not mapped to "no row" is a finding at the repair's site, saying the class, its ISO 32000-2
  clause and what was read instead. A departure the reader reads without repairing is outside it,
  which the rows ISO 19005 adds and the readers' refusals are for.
- **One table, `pdf_archive::repairs::MAPPING`**, maps every class by its stable name to section
  5.1 and to the rows it also touches — `syntax/integer-past-i64` to 6.1.13's integer row,
  `syntax/endobj-missing` to 6.1.9's object-syntax row, the stream-keyword and stream-extent
  classes to 6.1.7.1's rows, the encryption classes to 6.1.3's — or to `NoRow` with its reason
  (`syntax/reference-chain-too-deep` and `syntax/scan-refused`, bounds of this reader that a
  conforming file can reach). The cross-reference classes map to 5.1 alone: section 6.1.4 states
  only the line ending after `xref`, which its row reads from the bytes. A class the table does
  not name is read as 5.1 alone, and the closing gate (fix round 5) forbids it.
- **A row that reads the log reads the whole file first** (`Examination::read_whole`: every object,
  the content survey, and every stream's data, an image's codec excepted), once per report, so the
  answer does not depend on which rows ran before it.
- **6.1.9's `end_of_object` keeps returning on a tail window with no `endobj`.** A window that
  does not hold the keyword is not evidence it is missing — the gap to the next object may be
  longer than the window — and the parser's log is: the row reads `syntax/endobj-missing` instead.

## Consequences

- A damaged file the reader can read now fails section 5.1 where it failed nothing before. The
  archival converter answers the row as one the writer satisfies (`WRITER_EMITS`): its output is
  the repaired reading written in the serializer's syntax, and the input's failure is reported
  once. A repair a rewrite cannot undo stays in the output's own log, and the output's own check
  fails on it — repaired and stated where a remedy exists, never certified in silence.
- `read_whole` decodes every stream once more than a report did when no row asked for its data;
  it is the price of a log that is the whole file's.
