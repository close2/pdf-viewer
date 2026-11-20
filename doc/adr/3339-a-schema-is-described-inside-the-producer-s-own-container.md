# 3339 — A schema is described inside the producer's own container

Session 2159. Status: accepted. Context: ISO 19005-2 sections 6.6.2.3.2 and 6.6.2.3.3, ISO 19005-4
section 6.7.9; ISO 16684-1 clause 6 (a property name is unique within its packet); ADR 1245 (the
container this conversion writes), R14 of `doc/pdfa-blocking-issues.md`.

## The question

`JPXTestGrey` (pdfbox) stops at 2b under every profile; printed at the refusal: "this packet states
an extension schema container that does not describe every schema it uses, and this conversion
writes a container as a description of its own rather than merging into one a producer wrote".
Adobe Illustrator CS5 wrote a PDF/A-1-style container describing `pdf` and `xmpMM`, and the packet
uses `illustrator`, which it does not describe.

## The decision

Section 6.6.2.3.3's Table 1 makes the container's value a bag of schemas, so a schema the producer
left out is described by one more item of that bag. `pdf_model::xmp::describe_into_container`
finds the one `pdfaExtension:schemas` whose value is one `rdf:Bag` — prefixes resolved through the
bindings in scope, not by spelling — and inserts each schema item before the bag's close tag; each
item declares the three bindings it uses, so the producer's prefixes elsewhere change nothing it
means. Every other byte crosses unchanged, and the packet is read back as before. A packet stating
two containers, or one with no bag, still refuses by `CONTAINER_ALREADY_THERE`, reworded to say
that. The fields the conversion supplies are ADR 1245's, unchanged; nothing new is decided.

## Result

`JPXTestGrey` converts under both shipped profiles at 2b and 4, veraPDF 1.30.3 passing, page 1 at
72 dpi pixel-identical. A fixture with a producer's container describing one schema and a property
of another conforms, with one container holding both descriptions.
