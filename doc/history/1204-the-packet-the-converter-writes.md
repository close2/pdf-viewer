# 1204 — The packet the converter writes

The corpus-named round of batch thirty-one, on `doc/todo/66`'s metadata family: the five answers
`only-metadata-loss` gave at PDF/A-2b that no version carried out.

## What was built

**The packet replacement** (ADR 1245). `pdf_archive::packet_faults` and `metadata_streams` read
the three ISO 19005-2 section 6.6.2.1 rows' own judgements as a population — ADR 1234's rule, and
the reason is the same one: a findings list is capped where a document's metadata streams are not.
`Rewrite::FreshMetadataPacket` puts a conforming packet in place of one this tree cannot edit by
span, and it is the *first* of five writers over a packet, the other four skipping a stream it
replaced. `Loss::MetadataPacket` is the authorisation; `remedy = "preserve"` with
`original = "page"` puts the producer's own bytes on ADR 1014's appended page instead.

**The extension schema container** (ADR 1245, section 4). `pdf_archive::undescribed_schemas` says
what the packet states about a schema nothing describes; `pdf_model::xmp::describe` writes the
container. Three of section 6.6.2.3.3's fields exist in no file — the schema's name, each
property's category and description — so each is one fixed sentence saying so, the category is
`external` because this converter derived none of the value, and the decision is `Stated` rather
than `Mechanical` so that an operator is told. `undeterminable = "discard"` cuts a structured
property and describes the rest; without it that stops the run.

**The amendment identifier** (ADR 1246). Section 6.6.4's form is the number and the year separated
by a colon; neither half is recoverable from one that is not, the entry is optional, so
`Loss::AmendmentIdentifier` cuts it by span.

**And a reader bug the work found.** `Configuration::read` kept the first applicable row per site,
so a target-qualified row written *under* its unqualified default was dead text —
`only-metadata-loss` asked for an attachment at PDF/A-4f and silently got a page. Qualified rows
now win whatever order the file states them in, which made the 4f count honest rather than smaller.

## What is left, and where it is written down

`tools/state.sh remedies` prints it. `only-metadata-loss` answers every site it names at PDF/A-2b,
2u and 2a with a remedy this version carries out — the first profile to do so — and owes
`original = "attach"` at 4f and the two document-information-dictionary `preserve`s at every part 4
target. `as-if-printed` owes three at 2b: the two form sites and the embedded-file `discard`.
`doc/todo/66` and `doc/pdf-a-mitigations.md` sections 9 carry the detail.
