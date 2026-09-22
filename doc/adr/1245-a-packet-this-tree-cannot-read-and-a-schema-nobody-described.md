# 1245 — A packet this tree cannot read, and a schema nobody described

Status: accepted and **built**.
Context: `crates/pdf-archive/src/table/metadata.rs` (`packet_faults`, `metadata_streams`,
`undescribed_schemas`), `crates/pdf-model/src/xmp.rs` (`empty_packet`, `describe`),
`crates/pdf-transform/src/archive/{prepare,decision,rewrite,config,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s **metadata** family — `metadata/xmp-packets-well-formed`,
`-state-one-rdf-element`, `-meet-the-xmp-data-model` and `metadata/extension-schemas-embedded`,
which `doc/pdf-a-mitigations.md` section 9 catalogued and no version carried out.
Builds on: `doc/adr/1014` (the appended page and its bound), `doc/adr/1234` (a population read a
second time rather than off a capped findings list), `doc/adr/1233` (an answer that needs no
authorisation is already carried out), `doc/adr/0947` (nothing prepared that no failed requirement
asked for), `doc/questions/A48` (the repair this converter may not attempt).
Clauses: ISO 19005-2 sections 6.6.2.1, 6.6.2.3.2, 6.6.2.3.3; ISO 19005-4 section 6.7.2.1;
ISO 16684-1 sections 6.1 to 6.3 and 7.1; ISO 32000-2 §14.3.2.

## 1. Why these four go together

Four requirements, one subject: **what this converter may write into a metadata packet that its
producer did not.** The three packet rows are the case where the producer's bytes carry nothing
this tree can edit; the container row is the case where they carry everything except a
description. Both end at the same question, and answering it once is why they are one ADR.

## 2. The packet is replaced, and that is a loss

Four writers already edit a producer's packet **by span** — a header attribute cut, a property
removed, a container prefix moved, the identification schema restated — and every one of them
needs spans. A packet that will not parse has none; a packet stating two `rdf:RDF` elements has
no single place a description belongs; a packet breaking ISO 16684-1 clause 6's data model has
spans that mean something other than what they appear to. `doc/questions/A48` closes the repair,
because repairing is deciding what a malformed packet meant.

So the packet is **replaced**, and `Loss::MetadataPacket` is the word for it: everything the
producer recorded stops being metadata. What goes in its place is `xmp::empty_packet` — the
wrapper §14.3.2's own EXAMPLE prints, with one `rdf:Description` and no property — and **that is
all any of them states on its own.** The catalog's then gains the identification schema and this
conversion's recorded actions from the writer that already puts them there, so the one place in
this verb that states a conformance claim stays one place. Every other metadata stream keeps the
empty packet, because nothing in the file says what the unreadable one meant and composing
properties for an object's own packet would be writing the producer's metadata for them. The
stream stays, so every reference to it still resolves.

The replacement runs **first** and the four editors skip a stream it replaced. That is not an
ordering preference: an editor starting from the producer's original would write its result over
the fresh packet, and the output would carry the malformed bytes minus a property.

## 3. The original is kept where the operator asks

`doc/pdf-a-mitigations.md` section 9 found the third route and the owner's own words are in it —
*instead of losing metadata it could be appended or prefixed as an extra page*. `remedy =
"preserve"` with `original = "page"` and `fresh-packet = true` lays the producer's own bytes out
on a page appended to the document, under `doc/adr/1014`'s permission and every placement choice
`doc/adr/1025` section 4 argues. A stream whose data will not decode has no bytes to lay out and
is replaced with nothing preserved, named in the report all the same.

`original = "attach"` is **not built** and says so. Keeping the original as an embedded file is
PDF/A-4f's and PDF/A-4e's mechanism, it needs the attachment machinery, and a row asking for it is
named by `--remedy-sites` rather than quietly given a page instead.

## 4. The container describes shape, and says so in the fields it cannot fill

ISO 19005-2 section 6.6.2.3.2 requires every extension schema a stream uses to be described;
section 6.6.2.3.3's tables say with what. Four of the fields come out of the packet — the namespace
URI, the prefix, each property's local name, and the value type its own serialisation shows. Three
do not exist in any file: the schema's human-readable name, and every property's category and
description.

Each of those three is **one fixed sentence saying what it is**, not a name or a meaning this
converter made up for somebody else's schema. The category is `external`, and that is the one of
Table 4's two values that is true of what happened: the difference between the two is whether a
value is derived from the document's content, and this converter derived none of it. The decision
is `Answer::Stated` rather than `Answer::Mechanical` for exactly this reason — an operator is told
that the archive now carries a description of the *shape* of the producer's metadata and no
statement of what it means, and the report names every schema described.

The value type is a **form** rather than a type, which is the same coarseness the data-model row
already holds an array's items to: a serialisation shows that a value is text or an ordered array
of text and says nothing about whether the text is a date or a proper name. A structured value has
no form to read off — its fields would need a custom value type whose own field names are no more
in the file than the schema's name is — so it stops the run unless the row says `undeterminable =
"discard"`, which cuts the property and describes the rest.

## 5. Two refusals kept, and each is a clause rather than a limit

- **A packet already stating `pdfaExtension:schemas`.** ISO 16684-1 section 6.1 makes a property
  name unique within its packet, and this writer writes a description of its own rather than
  merging into a producer's. Two containers would break the data model, so the document is refused
  by name.
- **A packet still failing after the edit.** Both writers read the packet back and refuse where it
  does, which is `prepare_properties`'s rule: the writer says what it did and the packet says what
  it holds, and only the second is what a validator sees.

## 6. What this is not

It is not a licence to write metadata generally. The rule the two halves share is the one
`doc/adr/1014` states for a page: **what goes into the file is what the file already said, and
what the file did not say is named as a choice rather than filled in quietly.** Nothing here reads
a producer's intent, and the report is where an archivist sees which sentences are this
converter's.
