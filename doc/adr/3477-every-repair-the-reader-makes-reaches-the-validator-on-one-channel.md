# 3477 — Every repair the reader makes reaches the validator on one channel

Session 2194, the inventory round of the campaign. Status: **accepted** — the project owner
decided it on 2026-11-23; this record states the decision and the shape the fix rounds build.
Context: ISO 19005-2 and ISO 19005-4 section 5.1 and clause 6; ISO 32000-2 clause 7;
ADRs 3393, 3427, 3428, 3445 and 3446 (the validator's misses veraPDF found); the inventory,
`doc/reviews/2194-silent-repairs.md`.

## Context

The three reader crates, `pdf-syntax`, `pdf-model` and `pdf-font`, read malformed files as far as
they can: a cross-reference table is rebuilt by scanning, a stream's extent is found by its
`endstream` where `/Length` is wrong, a 34-digit integer becomes a real, an unreadable font
program becomes a substitute face, a wrong-typed operand turns its operator into nothing. That is
right for a viewer, whose job is to show what the producer meant. It is wrong for the PDF/A
validator, `pdf-archive`, which reads the same file through the same crates and so judges the
repaired reading rather than the bytes: rounds 2164 to 2190 found the validator's misses only
through veraPDF (ADRs 3393, 3427, 3428, 3445, 3446), each one a repair the validator never heard
of. The owner's instruction of 2026-11-23:

1. **An inventory round, run like an audit** — every place the three crates repair, recover,
   substitute or tolerate instead of reading what the file says; any repair that reaches the PDF/A
   validator without a report is a finding, with a failing test and a manifest entry; fix nothing.
2. **Fix rounds, by crate** — every repair emits a typed signal on **one channel** (existing reports
   reused where they fit); the validator turns each signal into a finding against the ISO 19005
   rule it touches; the viewer keeps repairing, but never silently.
3. **A closing gate** — a conformance test that fails if any repair variant has no validator
   mapping.

The aim is a validator that no longer depends on corpus files and veraPDF to find what it misses;
veraPDF stays an independent second opinion.

## What exists, and which of it reaches the validator

Surveyed in round 2194 (`doc/reviews/2194-silent-repairs.md` section 2 has the file and line of each):

| signal | crate | reaches `pdf-archive` |
|---|---|---|
| `filter::Damage` (from `decode_with_parms_reported`) | syntax | yes, for ASCII stages (`file-structure/ascii-data-ends-at-its-end-of-data-marker`) and font programs (`program_stopping_short`); every other decode uses the unreported API |
| `Lexer::integer_out_of_range`, `Lexer::salvaged` | syntax | content streams only, through `Runs::AsWritten` (ADR 3393); a dictionary's value never |
| `HexadecimalStrings` (odd or stray digits) | syntax | yes, `file-structure/hexadecimal-string-digits` re-lexes the bytes |
| `Document::was_recovered`, `CompressedRecovery`, `objects_lost_to_damage`, `misfiled_objects`, `padded_stream_keywords`, `cross_reference_entries_lost`, `cross_reference_chain_was_cut`, `damaged_dictionaries` | syntax | **no** — no row asks any of them; the rows that read raw bytes (header, `stream`/`endstream` line endings, object headers, `xref` line endings, trailing bytes) catch some of the same damage by their own reading |
| `Unsupported` (`StreamSyntaxFault`, `OperandShortfall`, `MissingResource`, `DamagedContentStream`, …), `ContentIssue`, `DamagedStream` | model | **no** — the survey walks tokens with `ContentReader::for_page` and never calls `take_issues` or the interpreter |
| `Page::substituted_media_box`, `Page::damaged_dictionary`, `Pages::found_by_scanning`, `CharProcsDamage` | model | **no** |
| `LoadedFont::is_substituted`, `states_no_descriptor`, `supplied_widths`, `FontDictRepair`, `substitute_for_unreadable` | font | partly — the font rows ask `is_substituted` and the program's own refusal; `FontDictRepair` and the program repairs (`sfnt::repaired_font_program`, the Type 1 trailer) never |

So the reports are many and typed, and almost none of them is the validator's input. The one row
that every repair breaches, **section 5.1's `conformance/adheres-to-the-base-standard`**, is
`Check::Unchecked`, and a verdict counts only `Failed` rows: a file the reader had to repair is
judged `Conforms` whenever the specific row the repair touches does not re-read the bytes.

## Decision

**One channel.** Every repair is a value of one type, recorded where it happens and read by the
validator from one place:

- `pdf_syntax::repair::Repair { class: &'static RepairClass, at: Location }`, where `Location` is
  an object identifier, a byte offset, or both, and `RepairClass` is a `static` descriptor — a
  stable identifier (`syntax/stream-extent-from-endstream`, `font/sfnt-loca-rewritten`, …), the ISO
  32000-2 clause the file broke, and one sentence of what was read instead.
- Each crate declares its classes as `pub static` items and lists them in one slice
  (`pdf_syntax::repair::CLASSES`, `pdf_model::repair::CLASSES`, `pdf_font::repair::CLASSES`), so the
  set of classes is enumerable without a registry crate and without `pdf-syntax` naming a font.
- The record lives on the `Document`, in a log beside the object cache (the same interior
  mutability, and for the same reason: reading a file is not changing it — `CLAUDE.md`'s
  immutability rule is about what the file says, and a log of how it was read is not that).
  `pdf-model` and `pdf-font` write to it through the document they read from.
- **The existing reports stay as the detail, and each also logs its class.** `Damage`,
  `DamagedDictionary`, `CompressedRecovery`, `Unsupported`, `ContentIssue`, `FontDictRepair` and
  the substitution records keep their callers; the log is the one place the validator reads, not a
  replacement for what the viewer already says.

**The validator's side.** `pdf-archive` maps every class to the rows it breaches: at least section
5.1's row, which becomes `Check::Implemented` over the log (every logged repair is a place the file
does not adhere to the base standard), and where a class touches a specific rule — section 6.1.7's
`/Length`, 6.1.13's integer bound, 6.2.11.4's embedded program — that row as well. A class the
validator deliberately does not fail (a repair of a reading the standard leaves to the reader) is
mapped to "no row" *by name*, with its reason.

**The closing gate.** A conformance test enumerates the three `CLASSES` slices and `pdf-archive`'s
mapping table, and fails on a class with no entry, an entry naming no class, or a class whose row
identifier is not in `pdf_archive::table::requirements`. A fourth check, by grep, keeps the census
honest: every `unwrap_or*`, `.ok()` and `.clamp(` on a value read from the file in the three crates
either sits beside a class or carries a `// not a repair:` comment naming the clause that makes the
default the specification's.

## Consequences

- The fix rounds run by crate, in dependency order — `pdf-syntax` (the channel and its classes),
  `pdf-model`'s content reader and page tree, `pdf-font` — then `pdf-archive`'s mapping, then the
  gate; `doc/reviews/2194-silent-repairs.md` section 6 has the order and each round's sites.
- The 2194 findings' failing tests are the fix rounds' acceptance tests; each passes once its
  class reaches the validator.
- The viewer's output is unchanged: no repair is removed. What changes is that a file the reader
  repaired can no longer be certified by the validator in silence.
- Costs: one log write per repair (on the error path only), one slice per crate, and a mapping
  table in `pdf-archive` that grows with every new repair — which is the point.
