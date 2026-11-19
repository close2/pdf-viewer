# Specification audit of ISO 32000-2 §12.8 — round 2128

The first of the project's own specification audits, asked for by the owner on 2026-11-19 ("start
audits") and made in the shape of the owner's review 2038
([`2038-specification-audit.md`](2038-specification-audit.md)). Baseline: main at `ae467539`.
Evidence: [`2128-specification-audit-12-8/`](2128-specification-audit-12-8/) — `findings.json`
(2038's schema), `expected-failures.json` (one failing test per behaviour finding). The same
entries are in `2038-specification-audit/expected-failures-current.json`, the manifest the gates
compare a nextest run with.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §12.8 and its subclauses (§12.8.1 to §12.8.7, Tables 255 to 264)
   was extracted from `doc/md/ISO_32000-2_sponsored_EC3.md` (lines 13129 to 13729): 232 modal
   sentences. Each was read in its paragraph, and Tables 257 and 259, whose value lists the
   conversion truncates, were read with that loss in mind (`signature.rs`'s `FieldSelection` doc
   names the same loss).
2. Writer obligations (how a signature *shall be created*, what a handler shall register) were set
   aside: this program creates no signature. What remained is what a processor reading a signed
   document owes: the signature dictionary's entries, the byte range, the three CMS sub-filters and
   PKCS #1, the PAdES dictionary and attribute rules and validation steps, the transform methods,
   the DSS and VRI, document timestamps, `/Perms` and `/Legal`.
3. Each was traced to the code that answers it in `pdf-signature` (`signature.rs`, `cms.rs`,
   `verdict.rs`, `timestamp.rs`, `signature/security_store.rs`), `pdf-pki`'s DER reader, and
   `viewer-core/src/notes.rs`, the report a reader sees. A finding is a sentence of the clause the
   code does not meet (B), a test whose expected value is not the clause's or that accepts the
   defect (T), or prose that says the code does what it does not (D).
4. Each B finding has a failing test named `*_spec_audit` whose expected value is read from the
   clause it quotes, with a control in the same test showing the fixture is what it claims. The
   signed fixtures are made by `crates/pdf-signature/tests/fixtures/spec_audit_2128/generate.py`
   with a fresh test key (no real certificate, no trusted list; A168).

What was found sound is not listed one by one; the count is the 232 sentences less the writer
obligations less the findings. Among what holds: the `/ByteRange` start, ordering and end-of-file
reading (§12.8.1, `coverage`, `excluded`, `signed_end`); the direct-object rule (`indirect_values`);
the PKCS #1 `/Cert` order (§12.8.3.2); twelve of §12.8.3.4.3's and §12.8.3.4.2's rules
(`pades_departures`); the signing-certificate comparison of §12.8.3.4.5 (a); the strict default of
`Acceptance` for §12.8.3.4.5 (d); the document timestamp's binding of its `TSTInfo` to its
signature (`timestamp::established`); Table 257's `/P` default and Table 258's implicit `FullSave`.

## Counts

| kind | count |
|---|---|
| B (behaviour) | 7 |
| T (test) | 2 |
| D (documentation) | 4 |

## Findings

**2128-B01 — an `adbe.pkcs7.sha1` digest is bound by no signature.** §12.8.3.3.1 puts "[t]he SHA-1
digest of the document's byte range" in the encapsulated content, and "[t]he digest of that
SignedData shall be incorporated as the normal CMS digest". `Signature::integrity` compares the
encapsulated content with the document; `authenticity_of` verifies the signature over the signed
attributes; nothing compares the `message-digest` attribute with the encapsulated content. So the
encapsulated SHA-1 can be rewritten after signing: the test edits one signed byte, writes the new
SHA-1 into the value, and the pair still reads `Unchanged` and `Verified` — which is what
`Verdict::reached` needs for its word. `timestamp::established` makes exactly this check for a
document timestamp (`ContentNotBoundToTheSignature`); the signature path does not. The
`timestamp_imprint` branch of `integrity` has the same shape for a non-timestamp signature carrying
a `TSTInfo`. Test: `pdf-signature::signature_value_spec_audit`
`an_encapsulated_document_digest_is_bound_by_the_message_digest_spec_audit`.

**2128-B02 — a detached value that signs encapsulated bytes vouches for any document.** For
`adbe.pkcs7.detached` §12.8.3.3.1 says "No data shall be encapsulated in the CMS SignedData field",
and §12.8.1 that "[t]he digest shall be recomputed and compared with the one stored in the
document". A value with an encapsulated content and no signed attributes gets
`Integrity::UnderTheSignersKey` (no `message-digest` to compare) and `Authenticity::Verified` over
`Signed::EncapsulatedContent`; `Verdict::reached` admits `UnderTheSignersKey` on the premise that
"question 2's answer *is* question 1's", which holds only for `Signed::TheDocumentsBytes`. The
document's bytes are never digested, so any CMS object a trusted signer ever produced, put in any
document, passes questions 1 and 2. This and B01 are the most serious findings. Test:
`a_detached_value_that_signs_encapsulated_bytes_does_not_vouch_for_the_document_spec_audit`.

**2128-B03 — §12.8.3.4.3 (a)'s content-type attribute is not read.** The rule is about "[t]he
attributes in the SignedData object": "content-type: shall be present and shall always have the
value \"id-data\"". `PadesDeparture::ContentTypeIsNotData` is computed from `encapContentInfo`'s
`eContentType`. A PAdES value meeting every other rule but carrying no content-type attribute has
an empty departure list. Test: `a_pades_value_without_a_content_type_attribute_is_a_departure_spec_audit`.

**2128-B04 — a `/ByteRange` that is not pairs of integers is read as one that is.** Table 255: "An
array of pairs of integers ... that shall describe the exact byte range for the digest calculation".
`byte_range` drops an odd trailing member (`as_chunks::<2>`) and every pair with a non-integer
member (`filter_map`), and answers for the pairs that remain: `Coverage::WholeFile` for a range the
file did not state. The function already refuses an over-long array whole, for the reason it gives
("a range with its tail cut off would describe a different digest"). Test:
`a_byte_range_that_is_not_pairs_of_integers_describes_no_range_spec_audit`.

**2128-B05 — a numeric `/V` in UR transform parameters counts as understood.** Table 258: "The
value shall be 2.2 . If an unknown version is present, no rights shall be enabled. NOTE This value
is a name object, not a number." `version_understood` is `as_name().is_none_or(..)`, so `/V 3` is
treated as absent and its rights are enabled. Test:
`an_unknown_usage_rights_version_enables_no_rights_whatever_its_type_spec_audit`.

**2128-B06 — a document timestamp is held to its whole-file range only under one sub-filter.**
Table 255 states the whole-file rule "[f]or document timestamp signatures", and §12.8.5.2 for the
DTS dictionary; the `ETSI.RFC3161` sub-filter is what such a dictionary "should" state.
`must_cover_whole_file` asks the sub-filter and not `/Type /DocTimeStamp`. Test:
`a_document_timestamp_must_cover_the_whole_file_whatever_its_sub_filter_spec_audit`.

**2128-B07 — the report calls the standard's long-term-validation shape a departure.** For a
`PAdES` or RFC 3161 signature with bytes after its range, `viewer-core`'s report says "this file
breaks that requirement" of Table 255. §12.8.1 says which file "the entire PDF file" is once there
are several revisions — the range ends at "the \"%%EOF\" comment ... terminating the incremental
update that adds the digital signature dictionary" — and §12.8.4.3 *requires* a DSS update to come
later than the signature's. The test appends exactly that update (the catalog with its `/DSS`,
written again) to a signed fixture and reads the sentence. `pdf-signature`'s `pades_departures`
makes the same reading (`RangeDoesNotCoverTheFile` from `coverage != WholeFile`), and the report
repeats it as a second sentence. Tests: `viewer-core::pades_appended_update_spec_audit`
`a_dss_update_after_a_pades_signature_is_not_reported_as_a_departure_spec_audit`, and
`pdf-signature::signature_value_spec_audit`
`a_revision_appended_after_a_pades_signature_is_not_a_short_range_spec_audit`.

**2128-T01 — a unit test's oracle accepts B01.**
`an_encapsulating_signature_is_checked_against_its_content_and_not_its_attribute` asserts
`Unchanged` for a value whose `message-digest` is deliberately not the digest of its content; a fix
of B01 that refuses such a value will fail it. The fixture's attribute should be the content's
digest, and the deliberately wrong one a second case expecting a refusal.

**2128-T02 — the whole-file departure sentence has no test.** `notes.rs:1418` is exercised in
neither direction; with B07's fix a later revision (no departure) and a range ending before its own
`%%EOF` (departure) each want one.

**2128-D01** — `Signed::EncapsulatedContent`'s doc says that for `adbe.pkcs7.sha1` "the same pairing
applies as above"; it does not (B01). **2128-D02** — `Verdict::reached`'s comment that question 2
answers question 1 for a `SignerInfo` without signed attributes is false where the content is
encapsulated (B02). **2128-D03** — `ContentTypeIsNotData` and `ProfileAttribute` describe
content-type's *presence* as checked (B03). **2128-D04** — ledger row 12.8.3.3.1 argues the
attribute need not be consulted "because for that format the attribute is a digest of the digest";
it is still what binds the content to the signature (B01).

## Ledger rows a behaviour finding contradicts

All 36 §12.8 rows are `implemented`. The findings contradict: **12.8.3.3.1** (B01, B02),
**12.8.1** (B02, B04), **12.8.3.4.3** (B03), **12.8.2.3** (B05), **12.8.5.2** (B06),
**12.8.3.4.2** (B07), and through them the aggregates 12.8, 12.8.3, 12.8.3.3, 12.8.3.4, 12.8.5.
They are not moved here; the fix round decides.

## Suggested fix groups

| group | findings | files |
|---|---|---|
| A — what a CMS value binds | B01, B02, T01, D01, D02, D04 | `pdf-signature/src/signature.rs` (`integrity`, `authenticity_of`, `Signed`), `verdict.rs`, `cms.rs` fixtures, ledger 12.8.3.3.1 |
| B — PAdES content-type | B03, D03 | `pdf-signature/src/signature.rs` (`pades_departures`, `PadesDeparture`), `cms.rs` (read the attribute) |
| C — dictionary reading | B04, B05, B06 | `pdf-signature/src/signature.rs` (`byte_range`, `usage_rights`, `must_cover_whole_file`) |
| D — the whole-file range across revisions | B07, T02 | `viewer-core/src/notes.rs`, `pdf-signature/src/signature.rs` (`pades_departures`) |

All four groups touch `signature.rs`, so one round takes them in the order A, C, B, D, or A alone first: A is the one that lets a forged or unrelated value reach the word *valid*.
