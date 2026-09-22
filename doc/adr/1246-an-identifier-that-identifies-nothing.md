# 1246 — An identifier that identifies nothing

Status: accepted and **built**.
Context: `crates/pdf-archive/src/table/metadata.rs` (`malformed_amendment_identifiers`),
`crates/pdf-transform/src/archive/{prepare,decision,rewrite,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s `metadata/identification-amendment-form`, which
`doc/pdf-a-mitigations.md` section 9 catalogued and no version carried out.
Builds on: `doc/questions/A48` (the repair this converter may not attempt), `doc/adr/1234` (a
population read a second time rather than off a capped findings list), `doc/adr/1245` (the four
writers this is the fourth of).
Clauses: ISO 19005-2 section 6.6.4.

## 1. The subclause leaves exactly one remedy

`pdfaid:amd` and `pdfaid:corr` are optional, and their value is fixed: the amendment or
corrigendum number and the year, separated by a colon. A value that is not of that form identifies
no amendment the subclause recognises.

Three things could be done with one and two of them are closed. **Correcting it** would mean
reading a number and a year out of a string that states neither in the form the clause gives — the
producer's intent, guessed, which is `doc/questions/A48`'s forbidden half. **Leaving it** fails
the requirement. What is left is **removing it**, and the entry being optional is what makes that
available at all: a file stating neither identifier conforms.

## 2. A loss, and a small one

`Loss::AmendmentIdentifier` rather than a mechanical rewrite, and the argument is not that the
claim was worth much. It is that a reader of the output cannot see the file ever made it. The
claim was already wrong in the standard's own terms — which is why the catalogue calls a departure
here hard to want — but *wrong* and *absent* are different facts about a document, and only the
report can carry the difference. So the report names the property and the value that was there.

## 3. A rewrite of its own, beside the identification schema

`xmp::restate` cuts every property in the identification namespace and writes this target's in
their place, so a conversion that restates the schema removes the amendment identifier as a side
effect. That is not a reason to leave this unbuilt: a document whose *only* fault is the form of
this one entry asks for no restatement of the part number, the conformance level or the revision,
and `doc/adr/0947`'s rule is that nothing is changed that no failed requirement asked for. So
`Rewrite::AmendmentIdentifierRemoved` cuts the entry by span with `pdf_model::xmp::remove`, every
other byte of the producer's packet crossing unchanged, and it is the fourth of `doc/adr/1245`'s
five writers over one packet — after the header cut, the property removal and the container
respelling, before the schema is restated into what it leaves.

## 4. The population is the requirement's own predicate

`pdf_archive::malformed_amendment_identifiers` reads one packet's bytes and returns the names
whose value breaks the form, each under the namespace spelling the packet actually used — which is
what a writer cutting by span needs, since the identification namespace has two admitted
spellings. The judgement is the requirement's own function, called from there, so this cannot cut
an identifier that row would have passed. The packet is read back afterwards and a document still
holding one is refused rather than left half-edited.
