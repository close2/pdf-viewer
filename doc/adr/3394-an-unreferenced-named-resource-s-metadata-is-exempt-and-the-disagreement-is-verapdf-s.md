# 3394 — An unreferenced named resource's metadata is exempt, and the disagreement is veraPDF's

Session 2173. Status: **accepted** (a reading; the remedy is named and not built). Context: ADR
3376 item 2, ADR 1021 (`pdf_archive::reach`); ISO 19005-2 sections 6.2.2 and 6.6.2.1 to
6.6.2.3.2; ISO 19005-4 section 6.2.2.

## What the object is

ADR 3376 called `PDFBOX-4532-reduced`'s `<< /Metadata 11 0 R >>` an object nothing reaches. It is
reached: the page's `/Resources /Properties` names it `/MC0`, and the page's one content stream
names no `/MC0` (its two `BDC`s write their property lists inline). It is a named resource whose
name the associated content stream never references.

## Decision

Both parts' section 6.2.2 end with the same exemption: such a resource is not used for rendering
and is exempt from the part's requirements (part 4 keeps 6.1.6 to 6.1.9; part 2, under TechNote
0010 A010, keeps 6.1.2 to 6.1.13). Section 6.6.2.3.1's rule about undescribed XMP properties is
none of those, and the metadata stream is part of the resource (it is reached only through it),
so the five undescribed `illustrator` and `xmpTPg` properties are outside the requirement.
Section 6.6.2.1's "all metadata streams" sentence is about conforming to the XMP specification
and well-formedness, which this packet meets. The validator's withdrawal stands; veraPDF checks
every metadata stream and does not apply the exemption, so its failure is its reading, recorded
with the others in `doc/pdfa-blocking-issues.md` section 1.

## The remedy, not built

A converter that wants veraPDF's agreement too can remove the exempt entry: nothing draws it, so
removing it costs no mark and no loss. That needs `pdf_archive` to say which entries the
exemption cut (`reach::Cuts` holds them) and whether a row's findings were withdrawn, and a
rewrite that removes those entries. It is queued, not a question: no owner's word is needed.
