# 2442 — A schema in a packet section 6.6.2.1 replaces takes the replacement's decision

Status: implemented. Session 1804, the archive slot of batch one hundred and twenty. Context: ADR
1245 (the fresh packet and the container), ADR 1270 (a requirement met by a remedy asked for at
another site), `doc/todo/72` section 4.

## The census

The census of real producers' files (`the_shipped_profiles_list_what_real_producers_files_refuse_on`)
under both shipped profiles at 2b and 4, ranked by documents per reason, put first the reasons no
remedy can carry under a profile's existing losses: `image-smoothing` and the `as-shown` font losses
(a profile's `stop`, `doc/todo/72` section 4 item 5), four fence rows (ADR 0816), part 4's
`DeviceCMYK` beside an RGB intent (a press profile is an operator's supply, ADR 2130), a CJK
composite font with no pack installed (the census runs without `QUORRA_CJK_PACK`), and a font no
shipped face has a glyph for. The largest a remedy could carry was twelve documents at 2b under
`metadata/extension-schemas-embedded`, every one refused with `prepare::NOT_ASKED_FOR` — the
placeholder whose own doc comment says it is never reported.

## The defect

Each of the twelve holds one packet that fails section 6.6.2.1 (the data model, one `rdf:RDF`, or
well-formedness) and uses an extension schema section 6.6.2.3.2 says is undescribed. The fresh
packet route replaces the stream with a packet stating no property (ADR 1245), and the container's
editor, reading what the editors before it left, found nothing to describe and returned the
placeholder; the table's `Stated` answer turned it into a refusal. A packet stating no property uses
no extension schema, so the replacement meets section 6.6.2.3.2 at that stream: the refusal named a
construction nothing needed.

## The decision

1. **`prepare_schema_descriptions` skips a replaced stream** as `prepare_properties` already does,
   and names it in `SchemasDescribed::replaced` where the validator found the requirement failing
   there (`schema_packets`, the row's own findings). The placeholder stays for a call where nothing
   was described and nothing replaced.
2. **Where every failing stream was replaced and none described**, `schemas_met_by_the_replacement`
   gives the row the decision section 6.6.2.1's row took — the same rewrite, the same loss under the
   same authorisation or `preserve`, so nothing is lost twice and nothing new is authorised; where the
   packet rows disagree, the one that stops the conversion. ADR 1270's Annex A.2 row is the
   precedent: a requirement met by the output because of a remedy at another site.
3. A replaced packet no decided row asks for (a departed packet row) refuses with a sentence saying
   so, `REPLACEMENT_NOT_DECIDED`, rather than with the placeholder.

## Consequences

- HEAD → this, pdf.js 2b (converted/conforming/refused/unreadable): `as-if-printed` 789/2/165/11 →
  792/2/162/11, `as-shown` 764/2/190/11 → 767/2/187/11; `doc/corpora` 2b `as-if-printed`
  348/6/120/16 → 352/6/116/16, `as-shown` unchanged (the four PDFBox files stop on a font loss the
  profile stops at); part 4 unchanged, which states no extension schema rule. Freed: pdf.js's
  `annotation-button-widget`, `-choice-widget`, `-text-widget` under both profiles and the four
  `PDFBox.GlobalResourceMergeTest` files under `as-if-printed`; none joins a refused list. Five
  stay refused for other reasons, now without the placeholder: three XFA files and
  `MOZILLA-1671648-0` (the XFA key, a signature), and `breast+cancer+immunotherapy`, whose
  well-formedness row refuses and whose schema row now names that refusal.
- veraPDF 1.30.3: the twenty outputs of the seven, both profiles where each converts, at 2b and 4,
  all PASS. Each output page drawn against its source at scale 2: every difference is one level
  (DeviceGray 98 drawn 99 in red through the output intent), the same at part 4, whose outputs are
  HEAD's byte for byte but for `stEvt:when`; the appended page carries the producer's packet.
- The other editors after the replacement (amendment, field descriptions) return the same
  placeholder on the same shape; no corpus document reaches them, and the same construction would
  serve each.
