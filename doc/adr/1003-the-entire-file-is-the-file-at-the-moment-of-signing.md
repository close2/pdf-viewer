# 1003 — "The entire file" is the file at the moment of signing, and the row that was not one predicate away is now a predicate

Session 982. Status: **accepted**. It settles the reading of ISO 19005-2 Annex B.1's first
sentence that ADR 0986 section 3 recorded and declined to guess at, promotes
`signatures/digest-covers-the-whole-file` to `Check::Implemented` on that reading, gives the
converter its considered answer for the row in the same commit, and takes the sentence-level
audit of `crates/pdf-archive` through both parts' annotation, form, signature and action
subclauses.

Context: `crates/pdf-archive/src/table/interaction.rs`, `src/coverage.rs`,
`src/table/file_structure.rs`, `crates/pdf-transform/src/archive/decision.rs`; ISO 19005-2
section 6.4.3 and Annex B.1; ISO 19005-4 section 6.5; ISO 32000-1:2008, 7.5.5, 7.5.6, 12.8.1
and its Table 252, 12.8.2.2.1; ISO 32000-2 §12.8.1 and Table 255; ADRs 0972, 0981, 0986.

## 1. The question, stated the way the last round left it

ISO 19005-2 Annex B.1 opens by requiring that a signature's digest take in every byte of the
file, the signature dictionary included, and leave out nothing but the signature itself, with
`/ByteRange` being where the file says so. Its NOTE 1 says the sentence makes a recommendation
of ISO 32000-1:2008, 12.8.1 normative; its NOTE 2 says the restriction ensures no byte of the
PDF is outside the digest but the signature. (The annex is licensed to a single reader —
`doc/questions/A16` — so this file paraphrases it and quotes nothing.)

`pdf_model::signature::Signature::coverage` reports `Coverage::WholeFile` for exactly the range
the first sentence describes, and two rounds therefore recorded the row as one predicate away.
ADR 0986 found the predicate rested on a sentence nobody had settled: in a file that carries an
incremental update after a signature, every signature but the newest reports
`Coverage::Unsigned`, and NOTE 2 reads, on its face, as if the annex forbade exactly that. So the
row was left `Unchecked` with three readings named — **(a)** a conforming file may not carry an
update after a signature, and every second signature fails; **(b)** it may, and the row's subject
is only the newest signature; **(c)** something else the annex states. A predicate written under
the wrong one fails conforming documents on *this crate's* reading, which is the one hazard that
moves `over`.

## 2. The reading, and the sentences it rests on

**The annex's sentence is about the moment of signing, and "the entire file" is the file that
exists at that moment.** Four things the standards state settle it, and none of them is the
annex's own text, which is the point: the annex makes a base-standard sentence normative and
inherits that sentence's subject.

1. **NOTE 1 names the sentence being made normative**: ISO 32000-1:2008, 12.8.1's bullet on the
   byte range digest, which says the range *should* be the entire file including the signature
   dictionary and excluding the value, and that other ranges are not recommended because they do
   not check for all changes to the document. Turning that *should* into a *shall* changes the
   modal verb. It does not change what "entire file" refers to, and 12.8.1 says what it refers
   to in its own NOTE 1: a signed document modified and saved by incremental update keeps the
   bytes the original signature's range covers, so that if the signature is valid the document's
   state *at the time of signing* can be recreated. The base standard describes an update after
   a signature as the ordinary case, with the signature staying meaningful.
2. **Table 252's `Changes` entry** says each signature results in an incremental save and later
   signatures have a greater length — in the edition PDF/A-2 adheres to, not only in ISO 32000-2.
   Two signatures in one file *are* an update after a signature; there is no other way to add
   the second.
3. **12.8.2.2.1** gives a certification signature's `P` values 2 and 3, which *permit*
   modifications after the signature — form filling, signing, annotation. A permitted change is
   an appended update, and its bytes are outside the certification signature's range by
   construction.
4. **ISO 19005-2's own section 6.4.3** permits a conforming file to contain the document,
   certifying and user rights signatures 12.8.1 permits — in the plural, and 12.8.1 has approval
   signatures *follow* a certification signature. A conforming PDF/A-2 file may therefore hold
   two signatures, and in such a file the first one's range necessarily stops where the file
   stopped when it was signed.

Reading (a) makes section 6.4.3's permission unreachable: a file could hold one signature and
never a second, and 12.8.1's "follow" would describe a file the part forbids. That is the
reading `doc/habits/reading-the-specification.md` says to reject when two clauses seem to
disagree — the one under which a file's own words mean nothing. Reading (b) is nearly right and
still wrong in a way that matters: the row's subject is *every* signature, each judged against
the file as it was when that signature was computed, not the newest alone. So the answer is
**(c)**, and it is what NOTE 2 actually says once "the PDF" is read as the file being signed:
what the annex forbids is the signer's choice of a *smaller* range — 12.8.1's "other ranges" —
and what it ensures is that no byte of the file as signed is outside the digest but the value.

ISO 32000-2 §12.8.1 states the same reading outright — in the multiple-signature case the range
runs from the header to the end of the `%%EOF` comment terminating the incremental update that
adds the signature dictionary, possibly followed by an optional EOL marker — and its Table 255
uses the phrase "the entire PDF file" for two sub-filters in the same clause without
contradiction. **That sentence is not the justification**, because the row binds PDF/A-2 and
ISO 32000-1:2008 does not state it (`doc/habits.md`'s rule, and session 975's eight shifted
numbers are why the rule exists); it is evidence that the base standard's own authors read the
2008 text the way the four points above do.

## 3. What the reading makes decidable, and the predicate

On a file in front of a validator the rule is four checks per signature, each one the annex's:

- the range starts at byte zero and has exactly two pairs — one gap, because what is excluded is
  the signature value and nothing else;
- the gap *is* the value: the bytes left out are the `/Contents` string, with or without its
  angle brackets. The standard says the value is excluded and does not say whether a hexadecimal
  string's delimiters are the value's bytes, so both are accepted — recorded as a choice;
- the signature dictionary is inside the range, which follows from the second check because
  12.8.1 makes the value a direct object of the dictionary;
- the range ends where a file ends: at the end of this file, or at the end of an `%%EOF` marker
  plus at most one end-of-line — where ISO 32000-1:2008, 7.5.5 and 7.5.6 end every revision —
  with more file after it. A marker followed by nothing but white space is not a boundary: those
  bytes were the signed file's own last line and they are outside the digest. A marker followed
  by an end-of-line and then more file is accepted whichever revision the end-of-line belonged
  to, because the bytes cannot say.

A range naming bytes past the end of the file is reported as one no digest over this file was
computed with, rather than as one that stops short.

**The population is the form's signature fields**, the walk
`signature_widgets_meet_the_annotation_rules` already makes, because 12.8.1 puts a signature
dictionary in a signature field's `/V` and section 6.4.3 requires it. This is not an economy: the
corpus holds a file — `6-1-12-t01-pass-a`, and its part 4 twin — whose signature is reachable
from `/Perms` alone, has no `/AcroForm`, and states a `/ByteRange` running 4810 bytes past the end
of the file. Under the field walk it is `signatures/signatures-use-signature-fields`'s finding
(still `Unchecked`) and not this row's, `over` stays 0, and no adjudication row was needed. The
population is the base standard's, not one chosen to keep a number at zero — but a round widening
it will meet that file, and should know that it is the *other* row the file breaks.

The predicate is `digest_covers_the_whole_file`, with eight tests built at the byte level: the
plain case, an update appended after the signature (0 findings), a range stopping before the
marker, one stopping at the marker with the file's own end-of-line outside it, one past the
file, a gap one byte off, a gap over the digits alone (0 findings), a second gap, and an
unsigned field (0 findings).

## 4. veraPDF, as evidence and not as the target

`CLAUDE.md` principle 5's direction of inference, applied: veraPDF's rule for this clause
(`PDFA-2B.xml`, 6.4.3 test 1, `doesByteRangeCoverEntireDocument`) was run on constructed
variants of the corpus's one field-reachable PDF/A-2 signature, and its output read. It passes
the signature with an incremental update appended after its revision's marker; it fails a range
ten bytes past the file, one ten bytes short, and one stopping at `%%EOF` with the file's own
`\r\n` after it. That is the reading of section 2 on every case, which raises confidence that
the sentences were read correctly. The one case where the two differ is the last shape *with an
update after it*, which this row accepts and veraPDF would presumably fail: section 3 says why
that case is undecidable from the bytes, and the difference is in the direction that cannot
move `over`. Its Java source was not read.

## 5. The converter's answer, in the same commit

Every `Implemented` row enters `pdf_transform::archive::unconsidered()` unless
`decision.rs` answers it, and `tests/archive_unconsidered.txt` is held to equality both ways. The
row is answered by a `REFUSED_BY_NAME` entry, `Because::NotBuiltYet(SIGNATURE_RANGE_IS_THE_SIGNERS)`:
a signature's range and its digest are one act only the signer can redo, no converted document
carries its source's signatures at all (`doc/pdf-a-conversion-limits.md` section 3.6), and what
the row waits on is the report section 3.6 asks for, which names each source signature and
whether it validated — the same thing the two 6.1.12 rows beside it wait on. The ratchet file is
unchanged and still empty.

## 6. The frontier, and what the third prefix found

The sentence-level audit now covers ISO 19005-2 clauses 6.3 to 6.5 and ISO 19005-4 clauses 6.3
to 6.6 — twenty-four subclauses — and the region read is still a prefix of each part in the
part's own order plus the annexes, which is the reviewability property ADR 0981 chose. The
figures are `cargo run -q -p pdf-archive --example frontier`'s.

What the pass found is that this tranche was already carried sentence for sentence, which is
the result ADR 0981 section 3.3 called a result. Three things are worth recording rather than
counting:

- **ISO 19005-4 section 6.3.3 has a row for a sentence it does not state.**
  `annotations/appearance-dictionary-present-from-base-standard` carries the have-an-appearance
  rule under that clause's number, and the clause's own text states only the `/N`-only rule; its
  NOTE 1 attributes the requirement to ISO 32000-2 §12.5.2 and Table 166. The reading carries a
  sentence saying so, in the shape ADR 0986 section 5 chose for `named-resources-are-defined` —
  the second such row, so the `Carried::Clarified` variant that ADR said the second one should
  buy is now owed and not bought here, because this one is not a clarification: the row is
  right and the clause number is where a reader would look for it.
- **The permission that decides Annex B.1 is in section 6.4.3**, and it is recorded there as a
  `StatesNoRequirement` sentence with the reason saying what it decides. A round that later
  revisits the annex's reading should find the argument's first premise at the clause that
  states it.
- **Section 6.5.2 of part 4 is one requirement and three conditionals.** Its second sentence
  binds every signature to one of the PAdES profiles; its third, fourth and fifth are conditioned
  on what the signer needs — a basic signature, validation after expiry, long-term validity —
  which no file states, and are recorded the way ADR 0972 recorded section 6.5.3's aspiration:
  as sentences that bind no file outright, with the reason saying which need each waits on.

## 7. Two smaller things

- **The single-signer row's reason was priced against the wrong word.** It said the row was
  closable from `SignedData`'s `signers` and `certificates`, and the count is; what is not is
  *DER-encoded*, because `pdf_model::der` accepts X.690's indefinite lengths on purpose and
  records rather than refuses them. A predicate written from that reader could say "parses as
  CMS" and not "is DER". The reason now says so, and what it waits on is in `pdf-model`.
- **Four citations in `table/file_structure.rs` spelled an ISO 32000-1:2008 clause with a `§`**,
  which `doc/habits.md` reserves for ISO 32000-2 and which the colon let past the scanner's
  number test (ADR 0987). They are spelled out. One of them was hiding a real difference: the
  comment quoted ISO 32000-1:2008, F.3.11's *shall not contain any entries other than `Size`* as
  if it were ISO 32000-2's, whose F.3.11 says *should*. The argument there needs only F.3.4, and
  the comment now says which edition says which.

## 8. The habit this leaves

**A reason that says a row is one predicate away has priced the route and not the clause, and
the clause half is the one a document can answer.** ADR 0986 found the unsettled sentence; this
round found that settling it took no new evidence — every sentence in section 2 was in a
document this tree already held, three of them in the base standard the row's own crate cites
constantly. The reading was expensive to *notice* and cheap to *make*. So the rule for the next
row of this shape: before writing "one predicate away", read the annex's NOTEs, then the base
clause the NOTE names, then that clause's own NOTEs — the standard usually says what its phrase
refers to within a page of using it.

## 9. What did not change

- **`over` is 0 on all six targets**, and the one standing miss (`6-6-2-3-3-t03-fail-b`, errata
  A029) is unchanged. The promoted row finds nothing in the corpus, for the reason section 3
  gives.
- **`crates/pdf-transform/tests/archive_unconsidered.txt` is unchanged and still empty.**
- **No requirement identifier was added, removed or re-keyed.** One was promoted:
  `signatures/digest-covers-the-whole-file`, `Unchecked` to `Implemented`. One reason was
  rewritten without changing its identifier: `signatures/signature-is-a-single-signer-cms-object`.
- `crates/pdf-model/src/signature.rs` was not edited. Nothing in the reading needs it to change:
  `must_cover_whole_file` records ISO 32000-2's *shall* for two sub-filters and is consistent
  with section 2, and `coverage`'s `Unsigned` is the right answer for a viewer asking whether
  the newest bytes are signed, which is a different question from this row's.
