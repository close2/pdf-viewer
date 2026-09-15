# 1105 — A place the output will not have

Status: accepted. Session 1091.
Context: `crates/pdf-model/src/appearance.rs` (`for_annotation`, `Written::drawn`,
`construct`'s `Screen` arm), `crates/pdf-transform/src/archive/prepare.rs`
(`prepare_appearances`, `Appearances::removed`, `APPEARANCE_NOT_DERIVABLE`),
`crates/pdf-transform/src/archive/decision.rs` (`answer_of`),
`crates/pdf-transform/tests/archive.rs`.
Builds: `doc/rfc/0007` section 2's `discard`, at the decision `doc/adr/1099` already argued —
this adds no remedy and no authorisation.
Clauses: ISO 19005-2 sections 6.3.1 and 6.3.3; ISO 19005-4 sections 6.3.1 and 6.3.3;
ISO 32000-2 §12.5.5, §12.5.6.12, §12.5.6.18, §12.5.6.24, Table 166, Table 171, Table 190.

## 1. What was measured, and what was taken

`crates/pdf-transform/tests/archive_corpus.rs` ranks the requirements that stop a conversion by the
documents each stops. Re-run after session 1085, its PDF/A-2b ranking reads
`metadata/extension-schema-container-fields` 15 — refused by ADR 1087 section 3's argument —
then `graphics/device-rgb-needs-a-default-or-an-rgb-output-intent` 12 and
`annotations/appearance-dictionary-present` 10.

**The colour row is `WRONG_FAMILY` and the sweep's `profile: None` is not why**, which had to be
checked rather than assumed: every one of its twelve witnesses already states a PDF/A output
intent whose destination profile is CMYK or grey, and ISO 19005-2 section 6.2.3 requires every
entry of one `OutputIntents` array to name the same profile object — so no RGB intent can be added
beside the producer's. Supplying `--output-intent-profile data/icc/sRGB2014.icc` converts none of
them, which was run. The row is the operator's own profile question and not a gap.

So the row taken is the appearance one. Reading its ten witnesses is what shaped the answer, and a
count could not have: **they are two faults wearing one requirement's name.** Three are annotations
of a subtype ISO 19005 does not admit — `Movie`, `Screen`, `3D` — which this converter *removes*.
Six are subtypes the part admits whose own clause states no artwork. One is a button widget, whose
refusal ADR 1099's neighbourhood already argues.

## 2. A requirement has no place on an annotation the output does not hold

ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 ask an appearance dictionary of the
annotations a **conforming file** holds. Section 6.3.1 of each forbids some subtypes outright, and
ADR 1099 built the removal that answers it: the reference leaves the page's `/Annots` and the walk
copies only what the converted document reaches. An annotation that has gone is therefore not in
the output, and the appearance requirement fails nowhere in it.

Until now the preparation asked `pdf_model::appearance` for artwork for every annotation the
*source* failed the row at, removed ones included — and refused the whole document because no
artwork could be had for a `Movie` or a `Screen`. That refused a conversion over a place the output
would not have. So `prepare_appearances` takes the removal's population and skips it, counting
what it skipped; where every failing place was skipped, `answer_of` gives the row the **removal's**
decision — `Loss::ForbiddenAnnotation`, `Rewrite::ForbiddenAnnotationRemoved` — rather than
`Decision::Stated` on a rewrite that constructed nothing.

**Nothing is constructed and nothing is authorised that was not already.** The answer is the one
the section 6.3.1 row gives for the same act, so a caller who authorises nothing is refused here
exactly as they are there, and the default run does not move by a document. That is what makes this
the least remedy that conforms: it is not a new remedy at all, it is the one already argued being
allowed to answer the second requirement it already satisfies.

**Why the condition is `at.is_empty() && removed > 0` rather than `at.is_empty()`.** A preparation
that found nothing to construct and skipped nothing is a disagreement between the requirement's
predicate and this population, not a removal — and attributing it to the removal would put the
wrong sentence in front of a person. That case keeps the behaviour it had.

## 3. The other seven keep their refusal, and one of them kept a false sentence

The six admitted subtypes — a stamp's legend, a caret, an unapplied redaction, a printer's mark, a
trap network, a watermark — are ADR 0816's fence, and `doc/pdf-a-conversion-limits.md` section 4.4
already says so: the artwork is the producer's and the file does not carry it, so constructing one
would put a mark on the page the document never described.

**What was wrong is the sentence, for nine of the ten.** `prepare_appearances` reported
`APPEARANCE_INCOMPLETE` — "construct only in part — a border style stating no highlight colour, a
caption whose room the entry does not give, a field value that would not lay out" — whenever
anything was owed, and `pdf_model::appearance::for_annotation` returns something owed both for a
construction that drew marks and fell short and for one that drew nothing at all. Every corpus
witness of this row is the second kind, so every one of them was told the first kind's reason. A
refusal is a question somebody can answer in advance (`doc/rfc/0007` section 0); a refusal naming
the wrong reason is a question nobody can answer. `Written::drawn` now carries the distinction and
the two refusals are two, with the accurate sentence — `APPEARANCE_NOT_DERIVABLE`, which also now
names a movie's poster and a watermark — on the arm the witnesses reach.

**And `Screen` had no arm of its own in `appearance::construct`.** `crate::annotation::construct`
answers it before reaching that function, so the viewer was right and only `for_annotation` — the
converter's entry point, which has no such caller in front of it — arrived at a catch-all whose
sentence is about an annotation that states no `/Subtype`. §12.5.6.18 answers it outright, in the
prose after Table 190:

> If AP is not present, the screen annotation shall not have a default visual appearance and shall
> not be printed.

The arm now quotes it. This is ADR 0901's correction reaching the second of the two paths, and it
changes no behaviour: the refusal was right and its reason was not.

## 4. What was not taken, and why

**An empty appearance stream for a `Screen`** would satisfy section 6.3.3 and change no mark, since
the clause states there is no default visual appearance. It is not written, because every target's
section 6.3.1 forbids the subtype anyway, so the annotation goes; a construction nothing needs is
one nobody has to argue about.

**A loss for the six admitted subtypes** — take the appearanceless annotation off the page as
section 6.3.1's is taken — would convert eleven more corpus documents and is the obvious next
question. It is not taken here because it is a *new* Ask about content no clause forbids: a
redaction's `/QuadPoints`, a stamp's `/Name` and an annotation's `/Contents` are the producer's,
and losing them needs its own argument rather than this one's. The refusal names them today, which
is what lets that argument be made from evidence.

## 5. What it did, measured

The corpus figures are `tests/archive_corpus.rs`'s to print. What they show is that the documents
gained convert with the *same* authorisation ADR 1099 already asked for and no other, so the
default run at every target is unchanged and only the authorised run moves — which is what an *Ask*
means, and is the difference from a new remedy.
