# 2412 — A prefix no declaration binds makes a packet not well-formed, at both parts

Status: implemented. Session 1789, the archive slot of batch one hundred and nineteen. Context: ADR
2394 and `doc/history/1780`, whose veraPDF run found the gap; ADR 1245 (the fresh packet); the
reading in `crates/pdf-archive/src/table/metadata.rs`'s module comment that keeps the data-model row
at part 2.

## The gap

pdf.js's `issue20453.pdf` carries a packet whose second description declares `pdf` and writes
`dc:title` and `dc:description` with no `xmlns:dc` anywhere in scope. Converted to part 4, veraPDF
1.30.3 fails it on 6.7.2.1-4, 6.7.2.1-5 and 6.7.3-1 — its XMP parser refuses the whole packet, so the
identification schema is never read either — while this validator passed it: `pdf_model::xmp` reads
the name into no namespace, and the only row that noticed, `metadata/xmp-packets-meet-the-xmp-data-model`,
binds part 2 alone.

## The brief's hypothesis, and why the row it named stays where it is

The brief asked whether the data-model requirement binds part 4. It does not, and the module
comment's reading stands: ISO 19005-2 section 6.6.2.1 asks a metadata stream to conform to the XMP
specification *and* asks its packets to be well-formed, while ISO 19005-4 section 6.7.2.1 asks only
that the packets be well-formed as ISO 16684-1 defines it. ISO 16684-1's clause 6, the data model,
is what conformance adds.

The fault is nonetheless part 4's, through the sentence part 4 does state. ISO 16684-1 section 7.1
requires serialised XMP to be well-formed XML and well-formed RDF, and an XMP reader to follow both
specifications' rules. The RDF/XML Syntax Specification's section 6 does not take a document that
fails to conform to Namespaces in XML, and Namespaces in XML 1.0 section 5's "Prefix Declared"
constraint requires every prefix but `xml` and `xmlns` to be declared on the element that uses it or
on an ancestor; its section 3 also forbids declaring a prefix to an empty namespace name. So a
packet using an undeclared prefix is not well-formed XMP under either part, and the row that already
binds both — `metadata/xmp-packets-well-formed` — is where it is reported. The data-model row keeps
its own sentence (a name in no namespace) at part 2, beside it.

## The decision

1. **`pdf_model::xmp::Xmp::undeclared_prefixes`** names every prefix used on an element or attribute
   that no binding in scope declares, once each, in first-use order, within the reader's property
   bound. It is a reading of the same walk as `rdf_elements` and `stray_character_data`, because the
   reader is where the bindings in scope are known; `Xmp::parse` and `parse_detail` are unchanged,
   so a viewer still reads such a packet's title (no output of theirs moved).
2. **`metadata/xmp-packets-well-formed`** records one finding per undeclared prefix, named `dc:`.
3. **`PacketFault::UndeclaredPrefix`**, whose requirement is the well-formed row, so the converter's
   existing route takes it: `Rewrite::FreshMetadataPacket` under the loss `metadata-packet`, with
   the profile's `preserve` page where the profile says so (ADR 1245). Declaring the prefix to the
   namespace ISO 19005-4's Table 1 suggests for `dc` would be deciding what a malformed packet meant,
   which A48 closes.

## What it costs

Two documents of the corpora stop being written as files veraPDF fails. issue20453 at part 4 meets
the answer it met at part 2: both shipped profiles ask for the producer's packet on an appended page,
and no font the document holds or this program ships can set it, so it is refused by name.
`breast+cancer+immunotherapy.pdf` writes `<dc:creator/>` directly under `rdf:RDF`, where this
reader interprets no property, so the data-model row never saw it and HEAD converted it at 2b and
4; it is now refused at 2b and 4 under both profiles, the appended page needing an output
intent the conversion does not add. veraPDF 1.30.3 failed HEAD's outputs of both on the packet rules.
`doc/history/1789` gives the counts.
