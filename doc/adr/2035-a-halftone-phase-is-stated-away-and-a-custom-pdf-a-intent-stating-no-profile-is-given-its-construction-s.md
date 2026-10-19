# 2035 — A halftone phase is stated away, and a `Custom` PDF/A intent stating no profile is given its construction's

Session 1599, the archive slot of batch ninety-five. Status: **accepted**, **built** in
`pdf-transform`. The two one-witness `not-built-yet` sites the veraPDF census ranks after ADR
2034's. Code: `crates/pdf-transform/src/archive/halftones.rs` (`phases`),
`crates/pdf-transform/src/archive/prepare.rs` (`SuppliedDestination`).

## 1. `graphics/no-halftone-phase-in-a-graphics-state` — a statement, not a loss

ISO 19005-2 section 6.2.5 forbids `HTP` in a graphics state, its note recording that PDF 1.3
removed the key. ISO 32000-1, which part 2 rests on, does not define it, and ISO 32000-2's Table 57
mentions it only in `HTO`'s NOTE as "the HTP key defined in PDF versions up to PDF 1.3 (1 st
Edition)". Annex I.3 says what an entry the specification does not describe does: adding one to a
dictionary does not change a processor's behaviour. So no processor of the base standard, press
or screen, reads the key, and removing it changes nothing any of them does — a statement under every
profile (`Rewrite::HalftonePhaseRemoved`), not ADR 2034's loss. The brief's hypothesis, that the
phase is a loss as the name is, did not hold: `HTO`, which PDF 2.0 *does* define, is that loss.
Witness `6-2-5-t01-fail-b` (`/HTP [0 1]`) converts to 2b under every profile, veraPDF 1.30.3
compliant, its page byte-identical at 72 and 720 dpi.

## 2. `graphics/pdfa-output-intent-states-a-destination-profile` — the entry is this verb's own

Part 2 and part 4 section 6.2.3 require a PDF/A output intent to state a valid ICC profile stream.
The witness, veraPDF's `6-2-2-t02-fail-a`, states `/S /GTS_PDFA1 /OutputConditionIdentifier
(Custom) /Info (CustomICC)` and no profile. Table 401 names `Custom` for "the intended production
condition is not a recognised standard", the profile then defining it, so this entry defines
nothing, and `pdf_model` — which draws through the first entry whose profile it reads — drew the
page as though no intent were stated. The entry is exactly what `intent_dictionary` writes,
`Custom` included, short of its profile. So it is given the profile that construction would name
(`Rewrite::DestinationProfileSupplied`, a statement): one another entry already names, which part
2's one-object rule makes every entry's, or else the profile a file stating no intent is given
(ADR 0948, ADR 1153's choice of family); its `Info` is the producer's and stays; and no second
PDF/A entry is appended beside it where the output intent rewrite is also wanted — the completed
entry is that intent. Refused by name: an entry naming something that is not a stream, an
identifier other than `Custom` (a registered condition's profile is the registry's, which this
program does not hold), a page-level array. The witness converts to 2b and 4, its page
byte-identical; veraPDF calls the 2b output compliant and the part 4 one fails only 6.7.3-1, the
identification namespace `pdf_archive` admits in either scheme (`IDENTIFICATION_URIS`), which
this change does not reach.
