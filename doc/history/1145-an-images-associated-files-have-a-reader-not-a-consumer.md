# 1145 — An image's `/AF` has a reader, not a consumer; §8.10.2's fallback is decided

2026-09-16. `batch-1141-1146`. Files: `crates/pdf-model/examples/required_entry_census.rs`
(one `/AF` tally), `crates/pdf-model/src/attachment.rs` (one fixture),
`doc/conformance/ledger.toml` (§8.9.5.1, §8.10.2, §14.13.7 notes/status), this file. No pixel
moved. Two near-done residues about associated files and missing resources, both resolved on
round 1121's §14.6.2 pattern: a *reader* general over any dictionary is not a *consumer* a
clause owes.

## Residue 1 — §8.9.5.1's `/AF` (§14.13.7)
§14.13.7's `shall` is the producer's — "the XObject stream dictionary shall contain an AF entry
whose value is an array of file specification dictionaries" — and §6.3.2.2 makes a rendering
processor owe no consumer of an image's associated files (interchange, §14.13.1).
`attachment::associated` already reads §14.13's array off any dictionary, an image's stream
dictionary included, so no image-specific reader and no consumer is owed — exactly object-level
metadata's shape (round 1121). Measured (trap 8): `required_entry_census` now tallies `/AF` and
counts **0 of 2997** image dictionaries over 963 pdf.js documents, **0** over the PDF 2.0,
format, PDFBox and differences corpora (450 opened, 4554 image dicts). Fixture (trap 13):
`attachment.rs::an_images_associated_file_reads_back_through_the_general_reader` plants an image
XObject `/AF` — the carrier the corpus never states — and confirms the general reader returns it.
§8.9.5.1 → **implemented**; §14.13.7 gains the fixture and the producer/consumer reading.

## Residue 2 — §8.10.2 (form XObject missing resources)
The residue is a decided behaviour, fully handled: a form stating no `/Resources` is read
against the *page's* dictionary (ADR 1059, taking ADR 1055 §5's open question), held by
`missing_resources.rs::a_form_nested_in_a_form_inherits_the_pages_resources_and_not_its_invokers`.
Every Table 93 entry that decides a mark is read; the unread entries describe the form, not its
appearance; the form-XObject `/AF` is §14.13.7's general-reader/no-consumer, not this row's; the
`/BBox [0 0 0 0]` pixel is §8.5.3.3.1's row's. Nothing else owed. §8.10.2 → **implemented**.

## Gates
(recorded in the report — worktree mid-flight with a sibling's `pdf-colour` extraction)
