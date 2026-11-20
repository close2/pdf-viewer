# 3230 — "The entire PDF file" is the revision the signature's update terminates

Session 2132. Status: accepted. Context: review 2128 findings B07, T02; ISO 32000-2 §12.8.1,
§12.8.3.4.2, §12.8.4.3, Table 255; ADR 3229.

## Decision

1. **`Signature::covers_its_revision(file)`** answers Table 255's "the ByteRange shall cover the
   entire PDF file" as §12.8.1 reads it once a file holds several revisions: "this range shall be
   the sequence of bytes starting from the "%PDF-" comment ... to the end of the "%%EOF" comment,
   possibly followed by an optional EOL marker, terminating the incremental update that adds the
   digital signature dictionary to the document." True for `Coverage::WholeFile`, and for
   `Coverage::Unsigned` whose signed bytes end at an end-of-file marker
   (`SignedEnd::AtAnEndOfFileMarker`); false for a range that stops anywhere else or is malformed.
2. **`pades_departures` reports `RangeDoesNotCoverTheFile` only where that is false**, and
   `viewer-core`'s report says "this file breaks that requirement" only for a dictionary
   `must_cover_whole_file` holds and whose range fails `covers_its_revision`. A later revision is
   said in the plain sentence ("N bytes were appended after that signature and are not covered by
   it").
3. **Both sides are tested**: `pades_range_end.rs` in `viewer-core` (a later revision, and a range
   shortened to end inside its `%%EOF` line), and `a_pades_range_that_stops_inside_its_revision_is_a_departure`
   in `pdf-signature`.

## Why

§12.8.4.3 requires the very tail the old reading reported: "If the signature dictionary to which a
VRI dictionary applies is itself in an incremental update section, the DSS/VRI update shall be done
later than the signature update." A reading of §12.8.3.4.2 that makes every long-term-validation
file a departure contradicts the standard's own prescription; §12.8.1's sentence resolves it.

## What is not judged here

What follows the marker is not inspected: a tail that is not a well-formed revision is the
revision reader's question (`revision.rs`), not the range's. One byte past a final EOL is therefore
no departure either, which is why `a_pades_signature_is_held_to_the_rules_that_need_no_certificate`
now expects three departures, not four. The document-timestamp chain's outermost-link rule
(`timestamp.rs`, `ChainRefusal::RangeDoesNotCoverTheFile`) still measures against the file's end;
§12.8.5's chain is its own reading and is left for a round that owns it.
