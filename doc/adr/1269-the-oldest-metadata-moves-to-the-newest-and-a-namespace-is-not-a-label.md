# 1269 — The oldest metadata moves to the newest, and a namespace is not a label

Status: accepted and **built**.
Context: `crates/pdf-model/src/xmp.rs` (`supplement`, `Supplement`, `Form`, `spelled_date`),
`crates/pdf-transform/src/archive/{prepare,decision,rewrite,config,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s **file structure** pair —
`file-structure/document-information-dictionary-needs-piece-info` and
`file-structure/document-information-dictionary-holds-only-a-modification-date`, which
`doc/pdf-a-mitigations.md`'s entry catalogued and no version carried out.
Builds on: `doc/adr/1212` (the merge's `/Info` writer and its §7.9.4 → ISO 8601 conversion),
`doc/adr/1245` (what this converter may write into a packet its producer did not),
`doc/adr/0947` (nothing is changed that no failed requirement asked for),
`doc/questions/A48` (the repair this converter may not attempt).
Clauses: ISO 32000-2 §14.3.3 (Table 349 and its ten NOTEs), §14.3.4, §14.5, §7.9.4, §7.9.2.2;
ISO 16684-1 sections 7.4 and 7.5; ISO 19005-2 section 6.6.2.3.1; ISO 19005-4 section 6.1.3.

## 1. The remedy is the standard's direction of travel, not a choice

ISO 19005-4 section 6.1.3 states two rules about the trailer's `/Info`, and this tree's ledger
carries a row for each: the entry may not be present unless the catalog states a `/PieceInfo`, and
a dictionary that is present may hold no entry but `/ModDate`. Neither says what to do with what
the dictionary held, because the base standard already has:

> Earlier versions of the PDF file format used the document information dictionary to represent
> document level metadata. In PDF 2.0 such use is deprecated except for two entries, CreationDate
> and ModDate . For any other document level metadata, a metadata stream (see 14.3.2 "Metadata
> streams") should be used instead.

and Table 349's NOTEs name the counterpart of every key one by one — `dc:title` for `/Title`,
`dc:creator` for `/Author`, `dc:description` for `/Subject`, `pdf:Keywords`, `xmp:CreatorTool`,
`pdf:Producer`, `xmp:CreateDate`, `xmp:ModifyDate`, `pdf:Trapped`. So the values move, the
dictionary goes, and nothing about either is this converter's opinion.

**The shapes are not a choice either**, which is the part that could have been got wrong. ISO
19005-2 section 6.6.2.3.1 requires a property to use its predefined schema *as defined*, so
writing `dc:title` as a simple value would fail `metadata/properties-use-known-schemas` on this
converter's own output. `pdf_archive`'s predefined-schema table is what records the shapes, and
§14.3.3's own EXAMPLE prints the two container ones — `dc:title` as an `rdf:Alt` with
`xml:lang="x-default"`, `dc:creator` as an `rdf:Seq`. The fixture is that EXAMPLE.

The decision is `Answer::Stated` rather than `Answer::Mechanical`, and section 3 is why.

## 2. `supplement` is additive because §14.3.4 permits nothing else

`xmp::restate` was the wrong tool and not a near miss: it removes every property in a namespace
before writing its own, so restating `dc:` to add a title would throw away the producer's
`dc:description`, `dc:rights` and `dc:language`. `xmp::supplement` is written beside it and does
one thing — it inserts a description holding the properties the packet **does not already state**,
just before the packet's `</rdf:RDF>`, with every other byte the producer's.

The rule is the clause's, and both halves of it:

> When writing modifications to an existing PDF document, if the PDF document contains time and
> date of creation only in the document information dictionary or in the document's metadata
> stream but not both, a PDF processor may add the information to the other, as long as both are
> fully equivalent.

> When writing modifications to an existing PDF document, if the PDF document already contains
> time and date of creation in both the document information dictionary and in the document's
> metadata stream, and the two are not equivalent, a PDF processor should leave the inconsistent
> values unchanged.

So the writer **has no way** to overwrite: the packet's own properties are read first and a
supplement whose property is already stated is dropped. That is the difference between obeying a
clause and remembering to. The report says which dictionary value went that way.

Dates cross grammars and nothing else does. `xmp::spelled_date` is ADR 1212's conversion lifted out
of `merge.rs` into `pdf_model::xmp`, where both writers can reach it: §7.9.4's
`D:YYYYMMDDHHmmSSOHH'mm` becomes ISO 8601's, every field §7.9.4 leaves out having a default the
clause itself states, and the zone — the one field with no default — absent where the producer
stated none rather than claimed as UT.

## 3. Two things an operator is told, which is why the answer is `Stated`

A reader's File → Properties panel reads from somewhere else afterwards. And a dictionary value the
packet contradicts is dropped rather than written, which is §14.3.4's instruction and is still a
value gone from the file. Both are in the decision's sentence and both are in the report, per entry.

## 4. The dictionary is kept exactly where the clause keeps it

A catalog stating a `/PieceInfo` keeps its `/Info`, holding `/ModDate` and nothing else. §14.5 is
the reason the carve-out exists rather than being a courtesy: a page-piece dictionary's
`LastModified` is compared against that date to "ascertain whether the data dictionary corresponds
to the current content of the document". Removing it would break a comparison the standard names.

**And then §14.3.4's fourth rule is in force**, because both sources are written:

> When writing the time and date of the most recent modification, typically when an existing
> document has been modified, a PDF processor shall ensure that the data in the document
> information dictionary and the document level metadata stream -if both are written -are fully
> equivalent.

So where the kept `/ModDate` and the packet's `xmp:ModifyDate` name different instants, the
conversion refuses by name. Choosing between them is deciding which of the producer's two
statements was true, which `doc/questions/A48` closes.

## 5. `unmapped = "extension-schema"` is recognised and refused, and the catalogue was wrong

`doc/pdf-a-mitigations.md`'s entry offered an operator a choice for a **custom** `/Info` key — a
key Table 349 does not define, whose value §14.3.3 makes a text string and for which no NOTE names
a property: an extension schema container, or dropping it. The first cannot be built inside this
project's own fence, and saying so is the finding of this round.

ISO 19005-2 section 6.6.2.3.2's container describes an extension schema **a packet uses**. A
`/Info` key is in no schema at all, so putting one into the packet means choosing a namespace URI
for it first — and a namespace URI is not a label on a property, it is the property's *identity*.
ADR 1245 section 4 allowed this converter to write one fixed sentence into each of the three
container fields no file holds, because a sentence saying *nobody stated this* is not a claim about
meaning. A namespace URI is. Nothing in the file states one, and the de-facto URI a large vendor
uses for this purpose is exactly the shape `CLAUDE.md` principle 5 forbids presenting as derived.

So the word is **read, validated and refused by name**, with `Configuration::unbuilt` listing it —
the same treatment `original = "attach"` had before ADR 1270 built it, and for a stronger reason:
that one was waiting on code, and this one is waiting on a fact no file supplies. `unmapped =
"discard"` drops the value and names the key in the report; `unmapped = "stop"`, which a row states
by saying nothing, keeps the refusal with the key named. Both shipped profiles that answer the site
say `discard`.

## 6. What this does not do

It does not write a `/Info` entry, anywhere, ever — the move is one-way, out of the dictionary and
into the packet. It does not reconcile the two sources for a document that is not being converted:
§14.3.4's reader sentence hands that discretion to the processor and §14.3.3's row records how the
viewer exercises it, which is to show both under separate headings. And it does not touch a part 2
target at all: ISO 19005-2 states no such rule, so a PDF/A-2 conversion carries the dictionary
through untouched, which is the catalogue's *retarget* answer and the cheapest one in it.
