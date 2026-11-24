# The PDF/A errata, each with the row it touches

The input to the next round's verdicts (step 4 of the owner's plan of 2026-11-24, ADR 3584): every
issue of `pdf-association/pdf-issues` carrying a `PDF/A-…` label or naming ISO 19005 or PDF/A in
its title, as `issues.jsonl` and `comments.jsonl` beside this file held them on 2026-11-24, with
its resolution as the comments record it and the `pdf-archive` rows whose clauses it names. Written
by this tree, not fetched; each issue's verdict is in `doc/errata-read-pdfa.md` (ADR 3590). "Agreed" quotes the comment that
records a working group's agreement; ISO 19005-4's errata page (`site/docs/19005-4-2020/
clause06.md`) publishes #77, #104 and #123, of which #104 and #123 carry `data-iso="approved"`.

| issue | state, labels | part / clause | resolution (decider, date) | pdf-archive row(s) |
|---|---|---|---|---|
| 76 | closed; wontfix, PDF/A-4, PDF/X-6, Parked | 19005-2/3/4 6.1.6.2 (Crypt filter) | wontfix: "will not be making any changes" (petervwyatt 2021-09-30) | file-structure/crypt-filter-is-identity |
| 77 | open; PDF/A-4, PDF/X-6, Parked | 19005-2/3/4 6.2.11.3.1 / 6.2.10.3.1 (CIDSystemInfo Supplement) | agreed: "PDF/A TWG agree to change text" (petervwyatt 2022-02-03), published on the errata page without ISO approval; the predefined-CMap question parked (2024-02-16). Applied, ADR 3582 | fonts/cid-system-info-agrees-with-the-cmap |
| 100 | closed; wontfix, ISO 21757-1, PDF/A-4 | 19005-4 B.3.2 (JS in PDF/A-4e) | wontfix / no change; assigned to TWG for record (petervwyatt 2021-07-30) | on-instantiate-script-only-on-explicit-user-action (B.3.2) |
| 104 | closed; PDF/A-4, ISO approved | 19005-4 6.2.9 (Annex Q reference) | published erratum: Note 2 added after 2nd par. (bdoubrov 2021-11-08; on errata page) | transparency rows with clause 6.2.9 (e.g. a-transparent-page-has-a-blending-space-or-an-output-intent) |
| 123 | closed; PDF/A-4, ISO approved | 19005-4 6.7.3 (pdfaid:conformance, http) | published erratum: "both errors ... need fixing" (petervwyatt 2022-01-13; errata page) | identification-schema-prefix, identification-declares-flavour-e/f |
| 129 | closed; wontfix, PDF/A-4, PDF/X-6 | 19005-4 / Extensions dictionary (general) | wontfix here: handled by aligned dated revisions (petervwyatt 2023-07-13) | none |
| 130 | open; enhancement, Parked, PDF/A-next, PDF/X-next | 19005-2/3/4 6.2.11.5 / 6.2.10.5 (width tolerance) | parked: "not for the dated revision" (bdoubrov 2023-11-13) | fonts/widths-agree-with-the-program |
| 167 | closed; wontfix, PDF/A-4, PDF/X-6 | 19005-4 6.2.7.3 (JPX colr box) | closed, no change here; returned to PDF/A TWG (petervwyatt 2022-04-07) | jpeg2000-uses-the-baseline-feature-set |
| 168 | closed; wontfix, PDF/A-4, PDF/X-6 | 19005-4 6.2.4.4 (Separation All/None) | closed no change in X side; "intentional" (petervwyatt 2022-04-07) | separation-alternate-spaces-obey-the-colour-rules-of-part-four, separations-of-one-name-agree |
| 192 | closed; proposed solution, PDF/A-4, PDF/UA-2 | 19005-4 6.3.2 (annotation flags, invisible sigs) | agreed, already in dated revision, not yet published (petervwyatt 2023-10-05) | flags-entry-present, printable-and-visible |
| 197 | closed; PDF/A-4, PDF/X-6, ISO approved | 19005-4 6.2.6 (rendering intents) | agreed: text added in A and X DRs, "Change applied" (lrosenthol 2023-07-26) | none for part 4 (rendering-intent-* rows are only_two 6.2.6) |
| 198 | open; bug, Parked, PDF/A-next, PDF/X-next | 19005-4 6.2.10.5 (width exception) | parked: "not for the dated revision" (bdoubrov 2023-11-13) | fonts/widths-agree-with-the-program |
| 213 | open; enhancement, Parked, PDF/A-next | 19005-4 6.6.1 (Launch for embedded files) | parked: "not for the dated revision" (bdoubrov 2023-11-13) | no-launch-multimedia-or-form-actions |
| 228 | closed; PDF/A-4, PDF/X-6, ISO approved | 19005-4 6.2.4.2 (ICC v4 with iccMAX tags); base-standard NOTE | agreed: NOTE 2 in ISO 32000-2 (petervwyatt 2022-12-15) | icc-profiles-carry-the-tags-their-version-requires |
| 231 | open; PDF/A-4, PDF/X-6, Parked | 19005-4 5.1 / 6.2.10.x (deprecated character collections) | parked with #77 (petervwyatt 2024-02-16) | no-deprecated-features; cid-system-info-agrees-with-the-cmap |
| 232 | closed; PDF/A-4, PDF/X-6, ISO approved | 19005-4 5.1 (deprecated 'F' operator) | agreed: explicit prohibition in both DRs (lrosenthol 2023-07-26) | no-deprecated-features |
| 244 | closed; PDF/A-4, ISO approved | 19005-4 clause 2 (XMP Part 2 link) | agreed: "TWG addressed this in DR" (lrosenthol 2023-08-09) | none (normative references, no row) |
| 245 | closed; wontfix, PDF/A-3, PDF/A-2 | 19005-2 B.1 (adbe.x509.rsa_sha1, NOTE 3) | wontfix: WG5 not touching A-2/A-3 (lrosenthol 2024-05-08) | signature-object-conforms-to-pkcs7 (part 2 B.1) |
| 253 | closed; PDF/A-4, ISO approved | 19005-4 6.7.3 (pdfaid:rev) | agreed: text rewritten (lrosenthol 2023-08-09) | identification-revision-year |
| 314 | closed; PDF/A-4, PDF/X-6, ISO approved | 19005-4 6.2.5 (TransferFunction, Type 5 halftone) | agreed: provision removed from A-4 (lrosenthol 2023-07-26) | halftone-transfer-function-only-where-required, second-transfer-function-is-default |
| 385 | open; documentation, PDF/A-4 | 19005-3/4 6.8 / 6.9 (AF, non-PDF associated files) | addressed in DR draft, awaiting final (petervwyatt 2024-09-11) | associated-file-media-type, relationship-stated |
| 391 | closed; ISO approved, ISO 32000-2 Amd1 | base standard Table 43 AFRelationship; touches 19005-4 6.9 | base standard, not a 19005 row (agreed rewording, petervwyatt 2024-04-25) | base standard, not a 19005 row (cf. relationship-stated) |
| 428 | closed; ISO approved | 8.7.3.1 tiling patterns; 19005-4 | no PDF/A change ("no actions", bdoubrov 2024-06-10); editorial note in 32000-2 | base standard, not a 19005 row |
| 554 | open; documentation, question, PDF/A-3 | 19005-3 6.8 (AF on StructTreeRoot) | question answered, no text change: "/AF entry can not be present" (bdoubrov 2025-06-09) | none (relationship-stated / associated-file rows are part 4 6.9) |
| 578 | open; bug, documentation, PDF/A-3, PDF/A-2 | 19005-2/3 6.6.2.3 (extension schema value types) | question under discussion (bdoubrov 2025-07-22), no decision | properties-use-known-schemas (part 2 6.6.2.3.1) |
| 598 | closed; bug, proposed solution, PDF/A-4, PDF/A-3 | 32000-2 Table 45 dates; 19005-3/4 6.8 / 6.9 | agreed: informative notes in ISO 32000-2 (petervwyatt 2025-10-23) | base standard, not a 19005 row |
| 624 | open; documentation, enhancement, PDF/A-next | 19005-4 6.5.1 (signatures, timestamps) | parked: "future versions" (bdoubrov 2025-11-10) | signing-does-not-break-conformance, signatures-use-signature-fields |
| 626 | closed; PDF/A-4 | 19005-4 6.1.1 (stale ref to 6.1.13) | agreed: "already fixed" in dated revision (petervwyatt 2025-09-23) | undescribed-data-never-renders |
| 630 | closed; documentation, wontfix, PDF/A-4 | 19005-4 A.2 (EmbeddedFiles) | wontfix / no change; DIS draft pointer (petervwyatt 2025-09-29) | pdfa-4f-carries-embedded-files |
| 631 | open; documentation, PDF/A-4 | 19005-4 6.2.3 (DestOutputProfile across OutputIntents) | agreed in DR: "adequately addressed" in DIS (petervwyatt 2025-10-08), unpublished | one-destination-profile-per-output-intents-array, page-output-intents-have-the-same-shape |
| 633 | closed; documentation, PDF/A-4 | 19005-4 6.7.2.3 vs 6.9 | agreed: "Resolved in ISO/DIS 19005-4" (petervwyatt 2025-11-11) | schema-associated-file |
| 635 | open; documentation, PDF/A-4 | 19005-2/3/4 6.2.11.3.3 / 6.2.10.3.3 (CMap and ToUnicode) | open discussion, no decision (myang-apryse 2025-10-15) | fonts/cmap-uses-only-predefined-cmaps |
| 841 | open; bug, PDF/A-4 | 19005-4 6.2.10.7 (Adobe-KR-9) | open: for next TWG (petervwyatt 2026-10-26) | fonts/to-unicode-values-are-usable |
| 842 | closed; duplicate, PDF/A-4 | 19005-4 clause 2 / 6.7.2.1 (ISO 16684-1 unlisted) | duplicate: "already fixed" in forthcoming revision (petervwyatt 2026-10-13) | none (clause 2 has no row); xmp-packets-well-formed for 6.7.2.1 |
