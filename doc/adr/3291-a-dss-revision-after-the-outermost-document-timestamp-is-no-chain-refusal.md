# 3291 — A DSS revision after the outermost document timestamp is no refusal of the chain

Session 2147. Status: accepted. Context: ISO 32000-2 §12.8.1, §12.8.4.3, §12.8.5.2, §12.8.5.3; ADR 3230
(`covers_its_revision`), ADR 1071 (the chain).

## The defect

`timestamp::chain` measured the outermost document timestamp's range to the file's end and refused
`RangeDoesNotCoverTheFile` for any tail, and refused `DoesNotCoverMaterial` for every DSS entry beyond it. A
§12.8.4.3 long-term-validation file is that shape on purpose: "If the signature dictionary to which a VRI
dictionary applies is itself in an incremental update section, the DSS/VRI update shall be done later than the
signature update."

## Decision

- §12.8.5.2's "The ByteRange key shall cover the entire PDF file" is read as §12.8.1 reads the entire file once a
  file holds several revisions — "to the end of the "%%EOF" comment, possibly followed by an optional EOL marker,
  terminating the incremental update that adds the digital signature dictionary to the document" — which is
  `Signature::covers_its_revision`. The outermost link is refused only where that is false.
- §12.8.5.3 asks that the material for the *previous* token be in the DSS before the new token, which then
  "will protect the whole structure". Material in a revision after an outermost link that ends its own
  revision cannot be the previous token's under that order and may be the outermost token's own, so it stays
  listed in `Link::material_uncovered` and is not refused. Where the outermost range ends no revision, nothing
  says where anything is, and every piece stays refused as before.
- Both sides are tested: `a_store_appended_after_the_outermost_timestamp_is_listed_and_not_refused` and
  `an_outermost_range_that_stops_inside_its_revision_is_refused_by_name`.

## What is not judged

Which token a piece of material serves is a certificate question; a file that appends the previous token's
material after the newest token breaks §12.8.5.3 and is not told apart here.
