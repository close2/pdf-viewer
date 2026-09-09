# 0931 — A clarification is not a correction, and it is not an opinion either

Session 941. Status: **accepted**. Recorded under the questions-directory rule: when an `A` file
appears, the round that acts on it records the decision in an ADR.

## The answer

`Q52` asked whether ISO 19005-2 section 6.6.6's `parameters` field should gate PDF/A-2 validation.
It was asked because three records disagreed and none of them could be read in an authoritative
form, so session 940 implemented the clause as published and put the question up. The owner
obtained **PDF Association TechNote 0010** (2017, *Clarifications of ISO 19005, parts 1-3 for
developers of PDF/A creators and validators*) on 2026-09-09 and answered on its strength:

> Its A021 resolution settles this question, and it is the blanket reading, accepted for the
> published parts […] So part 2's §6.6.6 does not gate PDF/A-2 validation — not parameters, and
> not action and when either. Part 4's §6.7.5 stands on part 4's own published text.

That is the narrow answer and it is carried out below. The decision this ADR records is the wider
one the question turned out to need, because the tree had no category for the document that
answered it.

## What the document is, and why neither existing category fits

`crates/pdf-archive/src/errata.rs` reads **corrections**. An approved erratum changes the
standard's text, so the corrected sentence is what a requirement means, and one entry there goes
further and withdraws a provision from a part altogether. `CLAUDE.md` principle 5 admits errata
without argument: a correction by the committee that wrote the sentence is part of the
specification.

TechNote 0010 is not that. It says so about itself in its own review section — it interprets the
existing specifications and does not change their text — and the working group is recorded as
having decided not to reopen the published parts. So there is no corrected sentence to implement.
A reader with ISO 19005-2 open will find section 6.6.6 exactly as this crate's row states it.

Nor is it the other thing the tree already knew how to refuse. `Q52` rejected two records — a
resolution reported inside a veraPDF corpus file's outline, and a conference summary on a web page
that contradicted it — and it was right to; principle 5 is explicit that another implementation's
reading is evidence about our reading and never the definition of correct. A round that had
treated either as authority would have been curve-fitting to veraPDF with extra steps.

**So the category is new: a normative clarification of a standard, published by the body that
publishes that standard's corrections, resolved by the working group responsible for it, changing
no text and changing what the text asks of a validator.** It gets its own module,
`crates/pdf-archive/src/clarification.rs`, rather than a row in `errata.rs`, because folding it
into the errata table would make this crate assert that a sentence had been corrected when it had
not — and the errata table is read by a person checking a verdict against a published standard.

## The readable test, which is the part that has to survive this round

`Q52`'s failure mode was not that it lacked judgement; it was that "committee guidance" had no
definition, so every record looked like a candidate. Four conditions, all required:

1. **This tree holds the document and a round has read it.** Not a summary, not a fact about it
   asserted in a third party's fixture. `doc/TechNote0010.pdf`, extracted to `doc/md/`.
2. **It is published by the body that publishes the standard's corrections.** `errata.rs` already
   records that the PDF Association's tracker states no errata pages for parts 1 to 3 and directs
   a reader to the Technical Notes instead. That pointer is the reason this is where those parts'
   corrections live rather than an opinion beside them — and following it is why this round could
   act where session 940 could not.
3. **The item carries a resolution of the ISO working group**, under the note's own resolution
   heading. The case above it — the problem statement, and the TWG's proposal where there is one —
   is not enough, and the note itself proves why: at four items the working group left the
   published requirement exactly as it stood after hearing the objection, in one of them declining
   a paragraph the TWG had proposed adding. A project that read the cases rather than the verdicts
   would have changed four rules the committee did not change.
4. **The item names the parts and clauses it reaches**, in its own pertaining line, so its reach is
   read rather than inferred.

Condition 4 is the one that does real work here rather than reading as a formality. A021 names
parts 2 and 3. It does not name part 4, and it closes with a note that the proposal was accepted
in principle for the part then being drafted — which was published in 2020 as ISO 19005-4, stating
`action` and `when` as requirements anyway. **A note about an intention and a published standard
are not the same kind of thing**, and where they disagree principle 5 reads the standard. That is
why part 4's row stays a check, and it is a reading rather than a concession to the owner's
sentence.

## What changed

- **`metadata/provenance-recorded-action-fields` reports nothing and says why.** Not deleted —
  deleting it would take the clause out of the verdict, and a reader who looked for section 6.6.6
  and found nothing could not tell a decision from an omission. Not `Check::Unchecked` either,
  which is a debt this crate owes and would be a lie: nothing is owed. It is a fourth state,
  `Check::OutsideValidation`, reported in its own section of the verdict with the record that put
  it there, and out of the coverage denominator for the same reason a processor's obligation is —
  no document can fail it.
- **`Judgement::clarified_by` carries the record**, exactly as `amended_by` carries an erratum, and
  for one further reason: a clarification leaves the text alone, so a reader who checked the clause
  and found the sentence intact would otherwise have no way to tell a reading from a defect.
- **`metadata/provenance-recorded-action-fields-four` stays a check**, on part 4's own text.
- **A029 is taken as well, and it settles `TN 0009`.** Session 940 ruled two corpus witnesses
  against the corpus because the record said to be contrary — TN 0009 on an extension-schema field
  that may be absent — returned HTTP 403. A029 is that reading in an authoritative form: an
  extension schema defining no custom value types may omit `pdfaSchema:valueType`, and a value type
  defining no structured fields may omit `pdfaType:field`, a validator allowing each absence and
  treating it as an empty array. It reaches those two field names and no others, so
  `6-6-2-3-3-t01-pass-e` now agrees and `6-6-2-3-3-t05-pass-a`, which omits `pdfaSchema:property`,
  stays ruled against the corpus. **The asymmetry is the clarification's own** and is the reason to
  read the resolution rather than the summary of it.
- **Every other item of the note is written down**, in `clarification.rs`'s module doc: two taken,
  five already true, seven owed with the row each would touch, one with no row at all. The note was
  read in full once; an item nobody recorded is an item the next round rediscovers.

## What it cost, which is one witness moving the wrong way

`6-6-2-3-3-t03-fail-b` omits `pdfaType:field` and the corpus names it a fail. A029 permits that
absence in as many words, so this crate no longer catches it and the sweep counts it a `missed`.
That is the honest column. The file *is* non-conforming — its own property carries a field no
value type describes — but for a reason under section 6.6.2.3.1 that this crate does not yet
check, because nothing here descends into a custom structure's fields. Keeping a rule A029
withdrew would have gone on producing the right verdict for a reason the committee has ruled out,
which is the shape of mistake this whole ADR is about.

Against it: three part 2 provenance witnesses that were adjudicated disagreements are now plain
agreements, `over` stays zero on all six targets, and `agreed` moved for the reason it is supposed
to move for.

## The licence, and why nothing here quotes the note

Page 1 of the document carries a copyright notice and no grant; its XMP packet carries no rights
properties at all. The publisher's own resource page returns HTTP 403 to this machine, as it has
since session 939, so its stated terms are unread — `doc/rfc/0006` records CC-BY-4.0 and a web
search agrees, but **a second-hand report of a licence is the same kind of evidence as a
second-hand report of a clause.** So the terms are treated as unclear and the consequence is a
rule: this tree cites the note by item number and paraphrases it, and quotes not one sentence of
it. `doc/third-party-data.md` carries the row and says that this is what was done and why.

## What it would take to change this

Publish an erratum. If the working group ever amends ISO 19005-2's text — rather than resolving how
to read it — the row moves from `clarification.rs` to `errata.rs` and the four conditions above
stop being the argument. Short of that, a later round may add an item from TechNote 0010's owed
list at any time, and does not need to reopen this decision to do it; what it may not do is act on
an item's problem statement or the TWG's proposal in place of the working group's resolution.
