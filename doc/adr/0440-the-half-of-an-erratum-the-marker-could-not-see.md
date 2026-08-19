# ADR 0440 — The half of an erratum the marker could not see

Status: accepted, 2026-08-19 (session 605). Amends [ADR 0426](0426-the-erratum-a-row-recorded-and-never-applied.md).

## Context

ADR 0426 built `spec-errata applied`, the sweep that asks whether a place *recording* an erratum
has *applied* it. Its noise is printed rather than filtered, and the noisiest shape by construction
is a correction quoting the wording it retired — so the output is sorted, with a **read-first list**
of hits carrying no mark of a correction in the four hundred characters either side.
[`applied::HISTORY`] is the list of marks.

The closing round of sessions 576–605 ran the sweep as `doc/todo/02` §4 asks and read the whole
read-first list. **Every hit in it that lived under `crates/` was correct writing**, and each named
the erratum with a verb `HISTORY` does not carry:

| the place | what it wrote | why it went unmarked |
|---|---|---|
| `structure.rs`, §14.8.6.1's default namespace | "**Errata Collection 3 replaced it** (Issue #151 …) with one that says where in the process the assumption is made" | `replaced` |
| `structure.rs`, §14.7.6.1's attribute object | "Issue #354 **replaces** *conforming product that owns* with *owner of*" | `replaces` |
| `attachment.rs`, §7.11.4.1 | the erratum's **replacement** quoted whole under the 2020 sentence | `replacement` |
| `type3.rs`, §9.6.4's named resources | "Issue #128 **replaces** it with §7.8.3's designated resource dictionary" | `replaces` |
| `border_precedence_census.rs` and `annotations.rs`, §12.5.2's `/BS` precedence | "which Errata Collection 3 Issue #287 **sharpens** to *shall be ignored*" | `sharpens`, `sharpened` |
| `write.rs`, §7.5.5's `startxref` | "**Errata Collection 3 has edited that sentence** … and the quotation above is the 2020 text" | `edited` |

The pattern is one sentence: **an erratum has two halves, and `HISTORY` only carried the verbs for
the first.** A writer recording what the collection *removed* — `struck`, `strikes`, `retired`, `no
longer` — was marked; a writer recording what it *put there instead* was not. Both retire the quoted
words equally.

The cost is not the hits. It is that a read-first list which is mostly correct writing stops being
read, which is `doc/traps/instruments-and-reports.md`'s trap 11 in the direction that does not fail
a build: a report whose condition is wrong in the *lenient* direction is loud, and a reader learns
to skip it.

## Decision

**`replace` and `sharpen` join [`applied::HISTORY`].** Both are bare stems for the reason `struck`
and `strikes` already are: this tree writes `replaces`, `replaced`, `replacement`, `sharpens` and
`sharpened`, and the window is a lower-cased substring test.

**`makes it` does not join it**, and that refusal is the load-bearing half of this decision. It
would have marked one more hit — `appearance.rs`'s "Errata Collection 3 makes it *shall be ignored*
(Issue #287)" — and it would also have marked **the defect this sweep was built for**, whose note
opens "Errata Collection 3 makes it enclosure (Issue #437)" and quotes the struck sentence three
sentences later. That is ADR 0426's own argument about borrowing
`conformance::blockers::HISTORY`'s `said` and `this row`, now with a second instance: **a phrase
saying an erratum changed *something* is not a phrase retiring *the quoted words*.**

The line is held by a test rather than by the doc comment alone, because it is an argument and an
array of strings does not carry one:
`applied::tests::a_phrase_that_only_says_an_erratum_changed_something_does_not_mark` plants the
§14.8.4.7.2 note in the `makes it` spelling and asserts it stays on the read-first list;
`a_correction_written_from_the_replacement_side_is_marked` asserts both new stems in both orders the
window reads.

Nothing is dropped by any of this. The sweep prints every hit and the mark is a **sort order**, which
is what makes widening it safe: a place moved off the read-first list is still in the output, one
section down.

## Consequences

- The read-first list falls from **22 to 10** over the same fourteen documents, and the two counts
  either side of it move with it (161 quotations of struck text unchanged; those reading as a
  correction 70 → 82). The ten that remain are five `crates/` sites whose verbs are rarer still
  (`now makes … rather than`, `read … until Errata Collection 3`, `states the key`), two ledger rows
  where the quoted phrase survives in the clause's *other* sentence — §12.7.5.2.2's push button
  "retains no permanent value" is the surviving definition rather than the struck one — and three
  dated ADR records, which are history by construction.
- **The list is not empty and is not meant to be.** A marker that matched everything would be a
  filter, and ADR 0426 chose to print rather than filter for the reason this round is an instance of:
  the sweep's founding defect reads like a correction three sentences above the stale quotation.
- The remaining five are a reading list for a round that wants one, not a debt: each was read here and
  each is a writer who recorded the erratum in a spelling nobody has needed twice. Adding a verb per
  sighting would end at a filter.

[`applied::HISTORY`]: ../../tools/spec-errata/src/applied.rs
[`applied::tests::a_phrase_that_only_says_an_erratum_changed_something_does_not_mark`]: ../../tools/spec-errata/src/applied.rs
