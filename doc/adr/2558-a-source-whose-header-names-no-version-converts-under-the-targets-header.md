# 2558 — A source whose header names no version converts under the target's header, its own kept in the history

Status: implemented, in `pdf-transform` (`archive::header`, `archive::version_for`,
`archive::prepare`, `archive::recovered`). Session 1862, the second archive slot of batch 127.
Context: ADR 2490's table (R06a, "decided here"), `doc/pdfa-remedies.md` section 3,
`doc/pdfa-blocking-issues.md` R06a, ADRs 1964 and 2498 (a rewrite whose record is a condition of
it), 2502 and 2523 (the structure this reader recovered, and what is the source's).

## The premise, and where the refusal was

The table's row `file-structure/file-header` was already `Answer::Mechanical(Rewrite::FileHeader)`.
Run at HEAD on `T01_004_header_invalid-syntax-no-dash` and `bug1606566`, the converter refused with
one sentence, `version_for`'s: the header states no version this tree reads, "and choosing one for
it would be inventing what its producer wrote". The same refusal stood in `recovered::build`, which
is what declined `issue9105_other` (`%PDF-1.`). The hypothesis, one declining sentence in the opening
path, held, with that second site beside it.

## The decision

1. **The output's header is the target's.** Where neither the header nor any revision's catalog
   states a version (`Document::version`, §7.5.2 and Table 29), `version_for` returns the version
   of the standard the part is built on: 1.7 for part 2 (ISO 19005-2 is built on ISO 32000-1) and
   2.0 for part 4 (ISO 32000-2 §7.5.2 has a writer of a file conforming to it identify the version
   as 2.0). Nothing is invented about the producer: the output is a file this program writes, and
   its header states the version of the file written, as every output's does. The validator then
   holds that file to the part's requirements, so the header is true of it. A source that states a
   version keeps every rule it had (a raise to part 4; no lowering of 2.x to part 2).
2. **The source's header bytes go into `xmpMM:History` and the report.** One entry, action
   `converted`, naming the header written, the target and the source's first line: the bytes before
   its first EOL, at most 64 (`MOST_HEADER_BYTES`, past which the entry says the line runs on),
   printable ASCII as itself, a backslash or quote escaped, every other byte `\xHH`, so the entry
   reads back to the bytes. The report's `recorded` field carries the same sentence.
3. **The record is a condition of the header**, as ADRs 1964 and 2498 have it for their rewrites: a
   packet that will not take the entry refuses `file-structure/file-header` by name
   (`NO_PLACE_TO_RECORD_HEADER`), and on a rebuilt structure, which fails no header requirement,
   stage 3 refuses the file. No loss word: nothing the producer wrote is gone from what a reader
   sees, and the bytes that are gone from the header are in the file.
4. **A rebuilt structure states the target's version too** (`recovered::rebuild` takes the target),
   and the history entry is read off the source, never off the rebuilt file. The rebuild now states
   `/Type /Page` on a page whose dictionary stated no type (Table 31 makes it required; every page
   it lists is a leaf of the tree it writes): `issue9105_other`'s direct page had none, and veraPDF
   could not parse the output without it.
5. **A page tree no conforming reader opens is declined as a source** (`page_tree`, the
   conversion's walk): a root that states `/Parent`, which Table 30 does not permit, and a `/Kids`
   entry that names no dictionary (a reference to nothing is §7.3.10's null). The structure this
   reader recovered is then written behind `recovered-structure`, its root new and its children the
   pages drawn; without the loss the source is refused by that sentence.

## What it frees, and what it does not

Converted and passed by veraPDF at both 2b and 4: the five `T01_00[3-7]` files, `bug1606566`, and,
under `recovered-structure`, `issue9105_other` and `PDFBOX-6040-nodeloop` (whose root names itself
as `/Parent` and lists a null kid). `pdf-archive` passed that tree as it stood; the checker's own
gap is named for batch 128.

## Rejected

- **Reading the header liberally** (`%PDF1.4` as 1.4). §7.5.2 fixes the form, and `Version::parse`
  says why a liberal reading would invent a number the file does not state.
- **A loss word.** The header is not content and a reader sees nothing different; the history entry
  is the whole of what the archive owes.

## Found at the merge

The no-pack census found one corpora document the base converted and this branch stopped with a
writer error, not a refusal: `jhove-errors/PDF-HUL-35/A Defence of the New Tenseless Theory of
Time.pdf`, "object 12 was already placed". Its node 7 states `/Count 10` over nine pages and a kid,
object 42, whose cross-reference offset lands inside a stream; the root states 13. Table 30's null
kid now declines the source and `recovered-structure` sends it to the rebuild, where
`pdf_model::Pages::get` skips node 7 by the ten it states and answers both index 9 and index 10
with object 46, so `pages_to_carry` listed object 46 twice and `build` reserved it twice
(`recovered.rs:207`, `Assembly::replace`). The rebuild now carries a page object once, at the
first index the reader draws it at, and the root's `/Count` of 13 against the twelve pages the
file holds is refused by name under ADR 2502's rule, at both targets and under both shipped
profiles: the page the file names and does not hold is not written as another page, and nothing
is said of it in silence. The test is
`recovered::a_page_the_reader_reaches_at_two_indices_is_carried_once_and_the_count_it_falls_short_of_refuses`.
Writing the twelve pages with the
thirteenth stated as lost would need a loss of its own, which is a question for the owner rather
than a fix.
