# ADR 0100 — A deletion is a copy of an object

Status: accepted, 2026-08-01.

## Context

`FILE_ONLY_EVIDENCE_CEILING` was 49 and the largest coherent block inside it was §7.5's: nine
`implemented` rows — §7.5.1, §7.5.3, §7.5.6, §7.5.7, §7.5.8 and its four subclauses — every one
of them naming `crates/pdf-syntax/tests/real_documents.rs`, which opens the fourteen specification
PDFs and asserts they open. That is a true statement about a file and no check at all on what the
rows claim, which is ADR 0098's whole subject.

§7.5 is also the family the hundred-and-seventh session worked in (ADR 0097), so the code was
fresh and the rows were not.

## What the audit found

**§7.5.6's rule was applied to two of the three things an update section can say.** The clause
lists them:

> A cross-reference section for an incremental update shall contain entries only for objects that
> have been changed, replaced, or deleted. Deleted objects shall be left unchanged in the PDF
> file, but shall be marked as deleted by means of their cross-reference entries.

and then states the rule this reader implements:

> When a PDF reader reads the PDF file, it shall build its cross-reference information in such a
> way that the most recent copy of each object shall be the one accessed from the PDF file.

`XrefTable::add` is that rule — sections newest first, first writer wins. But a *deletion* never
reached it: `read_classic_table` recorded an entry only where the keyword was `n`, and
`read_xref_stream` recorded only types 1 and 2. So a free entry was not an answer, it was a
silence — and the older section, which still holds the object's bytes because the clause says it
shall, answered instead. **The reader resurrected what the file had deleted.**

The same line of reasoning corrects §7.5.8.3's other sentence. Of an entry type this version of
PDF does not define:

> Any other value shall be interpreted as a reference to the null object, thus permitting new
> entry types to be defined in the future.

The row had read that as "skipped", on the argument that an object number with no entry resolves
to null anyway (§7.3.10). That is true of a *file* with no such entry and false of a section that
has one: skipping it hands the question to an older section, which is not the null object.

## Decision

**A cross-reference entry is `Option<Location>`, and `None` is a statement rather than an
absence.** An entry the reader has recorded as naming nothing blocks an older section exactly as
a located one does, because both are this section's answer. Absence — the number appearing in no
section at all — is still the missing key it always was.

Three consequences worth naming:

- `XrefTable::len`, `is_empty` and `object_numbers` skip deleted numbers. Every caller of
  `object_numbers` is walking the file asking each object a question, and a deleted number is not
  an object to ask.
- §7.5.8.4's hybrid ordering needed no change and is the one place §7.5.6's rule must *not*
  apply: "A PDF reader shall look in the cross-reference stream first, find the object there, and
  shall ignore the free entry in the previous section." The two compose for free, because the
  free entry lives in a section the ordering reaches *after* the stream, so it arrives at a
  number already answered. That is now a test rather than an argument.
- `entry_location` and its `Entry` enum exist so that a record shorter than `/W` promises
  (malformation, which stops the section) stays distinct from a record saying the number names
  nothing. An `Option<Option<Location>>` says the same thing and says it worse.

## What it cost, measured

Three of the 974 corpus documents write a deletion, and all three of them are the interesting
kind — the object's bytes are still in the file:

| | deleted | what it was |
|---|---|---|
| `prefilled_f1040.pdf` | 2005, 2006, 2007 | three form-field appearance streams, each with its own `/BBox` and `/Resources` |
| `issue13520.pdf` | 39 | an image XObject |
| `issue4800.pdf` | 2517 | an older copy that does not parse, so it was null either way |

**All three pages are byte-identical before and after**, checked by rendering page one of each
with the change stashed and restored. Nothing references what was deleted, which is what a
producer that deletes an object properly does. So the corpus cannot witness this rule, and the
whole of the evidence for it is the clause — which is trap 8 in its usual shape, and the reason
`tests/cross_references.rs` builds its files by hand.

Both gates confirm the absence of an effect rather than assuming it: 840 agreeing and 65
contradicted, 90 incomplete and 5 pageless, all unchanged.

## The tests, and what each can fail on

Ten, in a new `crates/pdf-syntax/tests/cross_references.rs`, one per row. Seven were confirmed to
fail by breaking the thing they guard — the free-entry recording, the unknown entry type, `/First`,
`/W`'s zero-width default, `/Index`'s `[0 Size]` default, `/XRefStm`'s position in the search
order, and the cross-reference stream's dictionary being the trailer. The other three are
structural: an object listed nowhere is unreachable, an object listed twice is read where the
table points, and the same document written as a table and as a stream is the same document.

Most are written as a *pair* differing only in the rule, because an assertion about one file
passes for a reader that never applies the rule at all — the shape
`junk_before_the_header_shifts_every_offset_in_the_file` established in the thirty-ninth session.
`an_object_stream_locates_its_objects_at_first_plus_their_own_offset` is the sharpest: the two
files differ only by a decoy object sitting between the header's last pair and `/First`, and a
reader that starts the first object where the pairs end reads the decoy *silently*, because the
decoy is a perfectly good PDF object.

## Consequences

`FILE_ONLY_EVIDENCE_CEILING` falls **49 → 40**. Four `implemented` rows have now been found wrong
inside that population and a fifth understated; this is the second one no page could have caught.

The lesson generalises past §7.5, and it is the one to carry: **a reader's answer of "nothing" is
data, and dropping it on the floor is not the same as recording it.** Both defects here were the
same shape — a case handled by not being handled — and both were invisible because the *next*
thing to run produced a plausible answer.
