# 0907 — A requirement written as a statement of fact, and three rows whose clause asks nothing

Session 933. Status: **accepted**. The eight rows of `doc/todo/01`'s next two blame bands, read
against the clauses, and what the twenty-fourth sweep's rank could and could not see.

## Context

ADR 0900 built `--bin permitted` and was emphatic that its output is a reading list rather than a
verdict: of 214 `partial` rows, 109 quote no requirement of the standard, and its own calibration
showed the rank silent on a row that owed a real obligation. ADR 0901 read five of them. This
session took the next tranche — `doc/todo/01`'s blame bands at rank 656 (`fc41aff8`) and 667
(`bad96d5f`), eight rows, every one of them flagged by the sweep in one of its five buckets.

**Three moved to `implemented`. Five kept `partial`, and four of those five had their stated
reason corrected.** The reason a row gives is worth as much as its status, and it is the half no
gate reads.

## Decision

### The three that moved: a clause that asks nothing of anybody

**§12.8.4.1** is one sentence with no modal verb in it — "Long term validation (LTV) of signatures
is achieved by using two types of dictionaries", the DSS and the DTS — and it hands each to a
subclause of its own. Both are read here. Its `partial` was carrying "counted, not parsed", which
is a true statement about this tree and a debt of §12.8.4.3 and §12.8.5, whose rows already hold
it. Carried on a framing sentence it made the ledger promise an unexecuted requirement where the
clause states none: **ADR 0896's shape one status over.**

**§12.8.4.5** states a permission and a recommendation and no `shall` at all: the validation data
"may be used by another party later relying on the signature", and "the preferred order of the
search for validation data should be as follows". The recommendation's antecedent is a *search for
validation data*, and nothing here searches — `Signature::authenticity` takes the signer's
certificate from the CMS object and says in its own words that a verified signature is not a valid
one, because the certificate arrived in the same file. The row is right about the tree and its
status was a claim nobody had made.

**§12.8.5.3** is ADR 0897's shape, in the last paragraph of a subclause rather than after a table.
The row explained the clause as expiry reasoning "a validator's" and never named the one sentence
in it addressed to this reader:

> When evaluating the DocMDP restrictions (see 12.8.2.2, "DocMDP") the presence of a document
> timestamp and/or DSS information shall be ignored.

This tree *does* evaluate those restrictions — `signature::permissions` reads the catalog's
`/Perms /DocMDP` and `restriction::asserted` decides an operation against Table 257's `/P` — and
consults neither a `/DocTimeStamp` nor a `/DSS` on that path. The requirement is met, and met by
construction rather than by a check, which is worth naming rather than assuming: the reading that
breaks it is the natural one, where a reader taking any incremental update after a certification
signature as a modification invalidates a document whose only later revision is the timestamp this
clause tells an archivist to add. The subclause's other two `shall`s bind whoever applies the new
token.

### The finding: a requirement can be written as a statement of fact

**§12.7.8.3.2 is flagged "quotes the standard and no quotation carries a modal verb", and the flag
is right about the words and wrong about the debt.** Every entry the row names as unapplied —
`/AP`, `/APRef`, `/IF`, `/A`, `/AA`, `/RV` — is stated `(Optional)` by Table 249, and the clause's
prose carries one `shall`, a writer's. What binds a reader is indicative:

> Unless otherwise indicated in the table, importing a field causes the values of the entries in
> the FDF field dictionary to replace those of the corresponding entries in the field with the same
> fully qualified name in the target document

*Entries* there is every entry of the table, so an unapplied `/AP` is that sentence unmet — a
requirement, declined — and not a permission declined. The row was already resting on exactly the
right sentence and is correct as it stands.

**So the sweep has a second blindness beside the one ADR 0900 recorded**, and it is the mirror of
the first. The recorded one is a row whose debt is a `can` in prose it never quotes (§12.7.7). This
one is a row whose debt is stated with no modal verb at all, because the standard sometimes writes
a requirement as a description of what an operation *does*. A rank built on modal verbs cannot
distinguish "the clause asks nothing" from "the clause asks it in the indicative", and the twelve
rows in that bucket are therefore the bucket to read by hand rather than the bucket to move.

### The four reasons corrected

- **§12.7.8.3.3's `/Rename`** was refused because the flag "cannot be applied by anybody, since the
  clause says outright that the flag does not define a renaming algorithm". The standard
  contradicts that in its next sentence. What Table 252 requires is an outcome — "If this flag is
  true , fields with such conflicting names shall be renamed to guarantee their uniqueness" — and
  the prose then offers an algorithm: "Although the Rename flag does not define a renaming
  algorithm, this might be implemented by a PDF processor renaming fields by prepending a page
  number, a template name, and an ordinal number to the field name." That is `CLAUDE.md`'s
  documented-choice case, not an impossibility. What actually makes it moot is the row's *other*
  refusal — Table 252's `/Fields` is read and not applied, so no field is imported to conflict.
- **§12.7.8.3.1** claimed "Table 246 whole, read, with three entries named rather than applied".
  The count is four (`FormsData::read` names `/EmbeddedFDFs` as well, an entry this row had never
  mentioned), and the clause states **Table 245** too, whose `/Version` is read into
  `FormsData::version` and never ranked against the FDF header — "If the header specifies a later
  version, or if this entry is absent, the document conforms to the version specified in the
  header", the same construction `Document::version` has carried for PDF since ADR 0207.
- **§12.7.8.3.4** described its gap as a design question about object spaces and never named the
  clause it departs from: Table 254's `/Page` is "The ordinal page number on which this annotation
  **shall appear**, where page 0 is the first page".
- **§12.8.2.1** was flagged as quoting nothing the conversion holds, and the clause is two
  sentences of which the first is the whole row: "Transform methods, along with transform
  parameters, shall determine which objects are included and excluded in revision comparison." A
  `shall` on a processor comparing revisions; this tree compares none.

## Consequences

- Three rows to `implemented`, five kept, five reasons rewritten. No code changed for this ADR.
- The reading list is not exhausted: the sweep's twelve-row "no modal verb" bucket now has a stated
  hazard, and `doc/todo/01` carries the next band.
- **A row read is not a row whose evidence was checked.** The rows here name tests inside
  `signature.rs` and `forms_data.rs` that do assert their clause; the annotation family read in the
  same session did not, and ADR 0906 has that count.
