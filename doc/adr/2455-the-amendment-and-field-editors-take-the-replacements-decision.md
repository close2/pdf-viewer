# 2455 — The amendment and field editors take the replacement's decision, and a fresh catalog packet is identified

Status: implemented. Session 1810, the editors half of the archive slot of batch one hundred and
twenty-one. Context: ADR 1245 (the fresh packet), ADR 2442 (the same construction for the schema
container), ADR 1246 (the amendment identifier's removal), ADR 2050 (the field descriptions).

## The defect

ADR 2442 found the schema container's editor returning `prepare::NOT_ASKED_FOR` — the placeholder
whose doc comment says no report prints it — on a packet ISO 19005-2 section 6.6.2.1 replaces, and
named two more editors that did the same on the same shape. Planted, both held: a catalog packet of
two `rdf:RDF` elements stating a `pdfaid:amd` that is not the number and the year separated by a
colon (section 6.6.4), and one whose value type leaves out the field its structure states (section
6.6.2.3.1), each refused its row with the placeholder. The editors read what the editors before them
left; the fresh packet states no property, so the cut found no identifier and the description no
field, and each returned the placeholder the table's answer turned into a refusal.

Under it a second defect: with the placeholder gone, both plants still wrote no file, because the
output failed section 6.6.4's part and conformance rows. ADR 1245 says the catalog's fresh packet
gains the identification schema from `prepare_metadata`, but the schema was restated only where a
row asked for it, and on a producer's packet that already stated its identification rightly no row
did — so the replacement took the identification away and the output check refused what it found.

## The decision

1. **`prepare_amendment` and `prepare_field_descriptions` skip a replaced stream** as the schema
   editor does, and name it (`Amended::replaced`, `FieldsDescribed::replaced`) where the requirement
   failed there — the amendment row's population is the catalog's one packet, and the field row's
   is the validator's own findings (`schema_packets`).
2. **`met_by_the_replacement`** (ADR 2442's `schemas_met_by_the_replacement`, widened) gives each of
   the three rows whose editor replaced and edited nothing the decision section 6.6.2.1's row took —
   the same rewrite, the same loss under the same authorisation or `preserve`. A packet stating no
   property uses no extension schema, states no amendment identifier and no structure field, so
   nothing is lost twice and nothing new is authorised: the `amendment-identifier` loss is not asked
   for beside `metadata-packet`. `REPLACEMENT_NOT_DECIDED` is reworded for all three.
3. **The identification schema is restated wherever the catalog's packet is composed afresh**, as
   ADR 1245 already said it was. Section 6.6.4 is a requirement of the output's packet, and the
   replacement is the output's packet; no loss attaches, because the values are the target's own.

## Consequences

- Three plants: the amendment, the structure field, and a replaced packet that stated only its
  identification; each takes the packet row's decision, converts when `metadata-packet` is
  authorised, conforms, and under both shipped profiles takes the profile's `preserve`.
- No corpus document reaches either editor's shape (ADR 2442). The identification half reaches a
  document whose catalog packet section 6.6.2.1 replaces and whose identification the source
  already met; the census before and after is in session 1810's record.
