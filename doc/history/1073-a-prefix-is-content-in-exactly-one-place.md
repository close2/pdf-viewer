# 1073 — A prefix is content in exactly one place

Date: 2026-09-15. Branch: `batch-1068-1073`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
ADR: [1087](../adr/1087-a-prefix-is-content-in-exactly-one-place.md).

## What the census ranked, and what the witnesses turned out to be

`tests/archive_corpus.rs` ranks the requirements that stop a conversion by the documents each stops.
Its top row at PDF/A-2b, the largest target, was `metadata/extension-schema-container-fields`,
refused with `Because::NotBuiltYet` — a gap named. The two colour rows tied with it at PDF/A-4 are a
profile the operator has not supplied (`WRONG_FAMILY`, the limits document's 10.1), which is not one.

Reading its witnesses one by one is what found the answer, and a count could not have: they are
**two faults wearing one requirement's name**. Most leave a field out of a container description.
Four state every field their table names, in that table's own field namespace, with the producer's
value, and spell it with another prefix. ISO 19005-2 section 6.6.2.2 says a prefix means nothing
*except* where one is identified as required, and section 6.6.2.3.3's four tables each identify one
— so that second fault is a spelling and nothing else, which makes it `Decision::Mechanical`.

## What was built

- `pdf_model::xmp::respell`: prefix tokens moved in the producer's own bytes, the `xmlns:`
  declaration that bound the old prefix with them. Refuses where the required prefix already means
  another namespace, where a default declaration carries it, or where the packet will not tokenise.
- `pdf_archive::extension_container_fields` / `ContainerField::misspelled` / `REQUIRED_PREFIXES`:
  the requirement's own predicate split into the two faults, so the converter cannot correct a name
  that predicate would have passed.
- `Rewrite::ExtensionSchemaPrefixes`, a `REMEDIES` row, `REFUSED_BY_NAME`'s row deleted; `Edited` is
  four writers over one packet, not three. An absent field refuses by name, naming the field.

## Calibration, and the two witnesses

Trap 13 at both levels: `a_packet_already_spelling_it_correctly_is_returned_byte_for_byte`, and
`a_container_already_spelling_every_field_correctly_is_not_rewritten` — the same hand-built fixture
spelled right conforms and is the identity conversion. Through `quorra-transform archive --to 2b`,
reopened and revalidated: `6-6-2-3-3-t01-fail-f.pdf` and `-t03-fail-f.pdf`, five places each, both
*conform*, the packet diff five prefix tokens and one `xmlns:` name. `-t02-fail-a.pdf`, whose
`pdfaProperty:category` is absent, is refused by name and writes nothing. The corpus figures are
the sweep's to print; what they show is that the documents gained convert **with nothing
authorised**, so the default run moves by as much as the authorised one — `Mechanical`, exactly.
