# 1270 — The second mechanism a `preserve` has, and the type a clause names for bytes nobody read

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/attach.rs` (new),
`crates/pdf-transform/src/archive/{config,prepare,rewrite,report,mod}.rs`,
`crates/pdf-model/src/attachment/filing.rs` (`embedded_file_stream_dated`),
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s **metadata** family's `original = "attach"` and its **forms** family's
`keep-xfa = "attach"`, which `doc/pdf-a-mitigations.md` sections 9 and 7 catalogued, which
`doc/adr/1245` named as not built, and which are the last two `only-metadata-loss` owed at part 4.
Builds on: `doc/adr/0814` (one writer for §7.11.4's objects, beside the reader that mirrors it),
`doc/adr/1211` (the one writer of §7.9.6's `/Names` node), `doc/adr/1245` (the replaced packet and
what may be written into one), `doc/adr/1257` (the `/XFA` removal and its dynamic predicate),
`doc/adr/1014` (the other `preserve` mechanism, and its bound).
Clauses: ISO 32000-2 §7.7.4 (Table 32), §7.9.6, §7.11.2, §7.11.3 (Table 43), §7.11.4 (Tables 44
and 45), §12.7.3 (Table 224), §14.3.3 (Table 349), §14.13.2, §14.13.3, Annex K;
ISO 19005-2 sections 6.4.2 and 6.8; ISO 19005-4 sections 6.4.2 and 6.9, Annex A, Annex B.

## 1. Two mechanisms, and the targets differ about one of them

`doc/rfc/0007` section 4.6.1's finding: the six targets differ about what may be *attached*, not
about what may be a page. ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 require an embedded
file to conform to a part of ISO 19005; Annex A lifts that for PDF/A-4f and Annex B for PDF/A-4e.
So bytes that are not a PDF stay inside the archive at those two targets and nowhere else, and
`preserve` is two operations an operator chooses between rather than one with a fallback.

The page half was built by ADR 1014 and 1245. This is the other half.

**The choice is honoured rather than approximated.** A configuration asking for the attachment at a
target that holds no file unchanged is an error naming both, not a quiet page: `check_rows` reads
the condition off the requirement table itself — the two rows requiring conformance are
`DERIVABLE`'s, and `requirement_binds` already knows which targets they bind — so a seventh target
added to `pdf_archive` would be classified without this code being edited.

## 2. What is attached, and the fence it is inside

Two sites, and the bytes are the producer's at both:

- **the XMP packet** `Rewrite::FreshMetadataPacket` replaces, under ISO 19005-2 section 6.6.2.1's
  three rows with `original = "attach"`. ADR 1245 replaced the packet because this tree edits a
  packet by span and one that will not parse has no spans; what the producer recorded stopped
  being metadata, and `original` is where the operator says what becomes of it;
- **the XFA resource** `Rewrite::XfaRemoved` takes out of the interactive form dictionary, under
  section 6.4.2's row with `keep-xfa = "attach"`. §12.7.3's Table 224 makes the entry "[a] stream
  or array containing an XFA resource, whose format shall conform to the Data Package (XDP)
  Specification", and Annex K says what the array is: "[a] packet is a pair of a string and stream.
  The string contains the name of the XML element and the stream contains the complete text of this
  XML element." So the resource is the streams end to end in the order the array states them, the
  first opening the document element and the last closing it — a reading of the clause rather than
  a repair. An entry that is neither shape, or one of whose streams will not decode, refuses by
  name: part of a resource is not the resource.

Neither is decoded, re-encoded, re-indented or re-parsed. That is what makes this `preserve` rather
than authoring, and it is `CLAUDE.md`'s provenance fence read at a new site.

## 3. The media type is a `shall`, and this round found it

The obvious worry was the media type. ISO 19005-4 section 6.9 requires an associated file's stream
to state one; the bytes come from a site whose entire subject is that this program could not read
them; naming a format would be asserting what it declined to interpret, which is exactly what
`embedded-files/associated-file-media-type`'s `supply` exists to keep it from doing. The round was
ready to write that down as a documented choice.

§14.13.2 states it outright:

> The embedded file stream dictionary shall include a valid MIME type value for the Subtype key.
> If the MIME type is not known, the value " application/octet-stream " shall be used.

So there is no choice here at all. The clause anticipated the case and named the value. **This is
the lesson `CLAUDE.md` already states about recording a silence** — read the titles around the
subject first — arriving at a site where the silence had not even been written down yet.

The same sentence answers `/Params` from the file rather than from a clock: an associated file's
stream "should contain a Params key whose value shall be a dictionary containing at least a ModDate
key whose value shall be the latest modification date of the source file". The source of these
bytes is this document, and §14.3.3's Table 349 is where a document states its own latest
modification date, in the same §7.9.4 grammar Table 45's entry is written in. So `/ModDate` is
carried across where the document states one and omitted where it does not; Table 45's
`/CreationDate` is left out, because it is "[t]he date and time when the embedded file was created"
and nothing says when a packet a document happened to hold came into being. `filing.rs` gained
`embedded_file_stream_dated` for that — the same writer with the two dates stated apart, which is
the only shape §14.13.2's sentence can be obeyed in.

## 4. `/AFRelationship` is `Source`, on Table 43's own definition

> Source shall be used if this file specification is the original source material for the
> associated content.

What is attached is in every case the producer's original of something this conversion replaced.
Table 43's NOTE 2 is why `Unspecified` would have been wrong — "Unspecified is to be used only when
no other value correctly reflects the relationship" — and this one does.

The specification goes into **§14.13.3's** catalog `/AF` array, which is where that subclause puts
a file associated with the document as a whole: a packet is the document's metadata and an XFA
resource is the document's form, so neither belongs to any page. §14.13.2 is the *embedded form*
and §14.13.3 the *place it is linked from*; this round had the two the wrong way round in four
comments until the conformance gate refused the quotation, which is that gate earning its cost
again. **The producer's own array is added to rather than replaced**, which is ADR 0947's rule.

## 5. One writer, three objects, one tree node

Nothing here writes a §7.11.4 stream or a §7.11.3 specification of its own: `filing.rs` is the one
writer of both (ADR 0814) and the one writer of §7.9.6's `/Names` node (ADR 1211), and `attach.rs`
calls it. What `attach.rs` adds beside them is Table 43's `/AFRelationship` and §14.13.3's `/AF`.

The tree is rewritten as one `/Names` node holding the producer's own entries and the new one,
which §7.9.6 permits outright and ADR 1211 argues. Where the document states the name dictionary as
an object of its own, the `/EmbeddedFiles` entry is rewritten *there* — so every other name tree the
producer wrote stays exactly where it was; where it does not, the catalog gains the dictionary. The
filing name carries the object number the bytes came from, because §7.9.6 compares keys "on a simple
byte-by-byte basis" and two replaced packets in one document would otherwise collide; a document
already filing something under that name refuses by name rather than losing one of the two.

## 6. Recorded in the file, and not a condition

Every attachment goes into the output's own `xmpMM:History`, with the sentence a person is owed and
the name the file is filed under. **Unlike the appended page it is not a condition of the remedy**,
and the difference is exact: ADR 1014's page exists by an amendment to `CLAUDE.md`'s authoring
exclusion whose fifth bullet makes the record part of the permission, and nothing here is composed
or invented — the producer's bytes move from one place in the file to another. So a packet that will
not take the entry loses the record and keeps the attachment, where it would lose the pages.

## 7. What this closes

`only-metadata-loss` now answers every site it names with a remedy this version carries out, at all
six targets — `tools/state.sh remedies` is what says so. `doc/todo/66`'s two milestones are both
reached. What is left in the metadata family is
`metadata/provenance-recorded-action-fields-four`, whose catalogue entry recommends a departure over
every remedy; what is left in the forms family is nothing.
