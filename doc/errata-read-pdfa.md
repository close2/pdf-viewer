# The PDF/A errata, read against this tree

A verdict for every issue of `pdf-association/pdf-issues` about ISO 19005 that `doc/errata/` holds
(the rule `doc/errata/pdfa.md` was written by, plus every `data-issue` on the site's ISO 19005
pages). It is the PDF/A half of `doc/errata-read.md`, which reads Errata Collection 3 against the
ISO 32000-2 quotations. `cargo test -p conformance --test pdfa_errata_verdicts` fails while an
issue in the snapshot has no row here, so a re-fetch that brings a new issue brings a failure
until somebody has read its thread (ADR 3590).

**The criterion is `CLAUDE.md` principle 5's sentence** (ADR 3581): an erratum the working group
that owns the standard agreed — published on pdf-issues.pdfa.org, or recorded as agreed in the
issue — corrects the clause, the corrected text governs, and the printed text is retired. A parked
or open issue, a question answered without a text change, and a `wontfix` are evidence, as any
other implementation's reading is.

**The verdicts are three.** `applied`: this tree implements the corrected text, and the row says
where. `not applicable`: nothing in the standards this tree validates against changes, and the row
says why. `recorded disagreement`: an agreed correction this tree does not apply, and the row says
why — there is none.

**One class needs its reason stated once.** Seven of the agreed issues (#192, #197, #232, #244,
#626, #633, #842) were agreed *into the dated
revision* of ISO 19005-4 (ISO/DIS 19005-4:202x, a revision of the standard under ISO ballot), and
the comment that records the agreement does not state the revised words. The site publishes no
erratum for them and they correct nothing in ISO 19005-4:2020, which is the edition
`pdf-archive`'s targets name and the one `doc/pdfa/` holds. They are `not applicable` to the 2020
edition, and they are owed again the day the revision is held: `doc/questions/Q1043` asks the owner
whether the DIS is to be held, and whether an agreement into a revision governs the edition before
the revision is published.

| issue | verdict | reason |
|---|---|---|
| #76 | not applicable | `wontfix` (petervwyatt 2021-09-30, "will not be making any changes"): 6.1.6.2's crypt-filter rule is unchanged; `file-structure/crypt-filter-is-identity` reads it as printed |
| #77 | applied | published on the ISO 19005-4 errata page (6.2.10.3.1, *less than or equal*), agreed by the PDF/A TWG 2022-02-03; `crates/pdf-archive/src/table/fonts.rs`'s supplement comparison and its tests, ADR 3582. The TWG decided on 2023-12-11 not to carry the change into the dated revisions and parked the issue for a white paper (2024-02-16), while the page still publishes it. The owner's A1043 keeps it applied, and every verdict a font's differing supplements decide carries a note saying so — `pdf_archive::errata::SUPPLEMENT_DIRECTION`, in the validator's text and JSON and beside the converter's decision (ADR 3605) |
| #100 | not applicable | `wontfix`, no change (petervwyatt 2021-07-30): B.3.2's script rule stands as printed |
| #104 | applied | published, ISO-approved: NOTE 2 to 6.2.9 points at ISO 32000-2 Annex Q, which is informative and is already the method `crates/pdf-archive/src/survey.rs` applies for part 4's page transparency (its Annex Q.2 conditions and tests); no predicate changes |
| #123 | applied | published, ISO-approved: 6.7.3's `http` namespace and the `pdfaid` prefix, with `https` accepted as the NOTE allows; `crates/pdf-archive/src/errata.rs` cites #123 on the four rows and `table/metadata.rs` reads both schemes |
| #129 | not applicable | `wontfix` here: the Extensions-dictionary alignment is left to the dated revisions (petervwyatt 2023-07-13); names no row |
| #130 | not applicable | parked for PDF/A-next, "not for the dated revision" (bdoubrov 2023-11-13): the width tolerance of 6.2.10.5 is unchanged |
| #167 | not applicable | closed with no change and returned to the PDF/A TWG (petervwyatt 2022-04-07): 6.2.7.3's JPX rules stand |
| #168 | not applicable | closed, no change, the Separation /All and /None behaviour "intentional" (petervwyatt 2022-04-07) |
| #192 | not applicable | agreed into the dated revision ("already been addressed in PDF/A-4 dated revision", petervwyatt 2023-10-05) without the revised words: the exemption of an invisible signature's annotation from 6.3.2's flag rules is not in ISO 19005-4:2020, so `annotations/flags-entry-present` and `annotations/printable-and-visible` stay as printed; owed when the revision is held (Q1043) |
| #197 | not applicable | agreed into the dated revisions of both PDF/A-4 and PDF/X-6 (lrosenthol 2023-07-26, "Change applied!"): a rule limiting rendering intents to Table 69's four that ISO 19005-4:2020 does not state, at a clause number only the revision gives; part 2's `rendering-intent-*` rows are unchanged; owed when the revision is held (Q1043) |
| #198 | not applicable | parked for PDF/A-next (bdoubrov 2023-11-13): 6.2.10.5's exception unchanged |
| #213 | not applicable | parked for PDF/A-next (bdoubrov 2023-11-13): 6.6.1's Launch rule unchanged |
| #228 | not applicable | agreed as NOTE 2 in ISO 32000-2 (petervwyatt 2022-12-15): an informative note in the base standard about private ICC tags; 6.2.4.2's requirement and `graphics/icc-profiles-carry-the-tags-their-version-requires` are unchanged |
| #231 | not applicable | parked with #77 (petervwyatt 2024-02-16): the deprecated character collections question has no decision |
| #232 | not applicable | agreed into both dated revisions (lrosenthol 2023-07-26, "Add an explicit prohibition of the 'F' operator"): ISO 19005-4:2020 states no such prohibition beyond its general rule on deprecated features (`conformance/no-deprecated-features`, unchecked); owed when the revision is held (Q1043) |
| #244 | not applicable | the XMP Part 2 normative reference's link, "addressed in DR" (lrosenthol 2023-08-09): clause 2 states no requirement a row checks |
| #245 | not applicable | `wontfix`, WG 5 does not revise parts 2 and 3 (lrosenthol 2024-05-08): ISO 19005-2 B.1 stands as printed |
| #253 | applied | agreed and ISO-approved: `pdfaid:rev` is the four-digit year of the revision; `crates/pdf-archive/src/errata.rs` cites #253 on `metadata/identification-revision-year`, whose predicate requires 2020 |
| #314 | applied | agreed and ISO-approved: the TransferFunction provision of 6.2.5 is removed from part 4; `crates/pdf-archive/src/errata.rs`'s `withdrawn_from_part_four` keeps the row from binding a part 4 target |
| #385 | not applicable | open, "addressed in DR draft", no final text (petervwyatt 2024-09-11): 6.9's associated-file rules stand as printed |
| #391 | not applicable | an agreed rewording of ISO 32000-2 Table 43's `AFRelationship`, the base standard's (petervwyatt 2024-04-25); 6.9's own requirement is unchanged |
| #428 | not applicable | no PDF/A action ("no actions", bdoubrov 2024-06-10); an editorial note in ISO 32000-2 8.7.3.1 |
| #554 | not applicable | a question about ISO 19005-3, answered without a text change ("/AF entry can not be present", bdoubrov 2025-06-09); part 3 is not a target here |
| #578 | not applicable | open, under discussion with no decision (bdoubrov 2025-07-22): ISO 19005-2 6.6.2.3's value types stand |
| #598 | not applicable | agreed as informative notes in ISO 32000-2 Table 45 (petervwyatt 2025-10-23); no ISO 19005 text changes |
| #624 | not applicable | parked for "future versions" (bdoubrov 2025-11-10): 6.5.1 unchanged |
| #626 | not applicable | a stale cross-reference in 6.1.1, "already fixed" in the dated revision (petervwyatt 2025-09-23): the reference changes, the requirement and `file-structure/undescribed-data-never-renders` do not |
| #630 | not applicable | `wontfix`, no change (petervwyatt 2025-09-29): A.2's embedded-file rule stands |
| #631 | not applicable | open; "adequately addressed in the forthcoming DIS" (petervwyatt 2025-10-08) with no words, and the reporter found the latest draft does not fix it: no agreed text exists to apply; the two output-intent rows read each array as 6.2.3 prints it |
| #633 | not applicable | resolved in the dated revision (petervwyatt 2025-11-11): the revision states no requirement on validating an associated schema; under the 2020 edition `metadata/schema-associated-file` is unchecked, so nothing this tree verifies changes; owed when the revision is held (Q1043) |
| #635 | not applicable | open, no decision (myang-apryse 2025-10-15) |
| #841 | not applicable | open, for the next TWG meeting (petervwyatt 2026-10-26) |
| #842 | not applicable | a duplicate: ISO 16684-1's absence from clause 2, "already fixed" in the forthcoming revision (petervwyatt 2026-10-13); 6.7.2.1's rule and `metadata/xmp-packets-well-formed` unchanged |

## What `spec-errata check` finds on the ISO 19005 page

`tools/spec-errata` reads the site's pages as its second input (ADR 3589). On ISO 19005-4's page it
finds six changes, four of which retire printed text: #77's *greater*, #123's `https` and its
`pdfa:conformance`, and #104's renumbered NOTE. `doc/pdfa/`'s conversion still carries three of them
as printed (#77, #104, #123's scheme), as a conversion of the printed standard must; the fourth,
Table 2's property name, is laid out differently there. No quotation, ledger note, document or
`pdf-archive` rule text lands on any of the four.
