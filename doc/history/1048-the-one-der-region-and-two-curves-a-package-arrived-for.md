# 1048 — The one DER region, and two curves a package arrived for

Session 1048, branch `batch-1044-1049`. Three residues of §12.8 blocked on neither a trust store
nor an unheld text. A predecessor with this contract died mid-edit on a quota; its Brainpool work
was in the worktree, uncompiled, and is finished here.

## 1. §12.8.3.4.2 — the tolerance now has the RFC's own boundary

RFC 5652 answers it in one paragraph of section 2: BER throughout — "each content type permits
single pass processing using indefinite-length Basic Encoding Rules (BER) encoding" — then "Signed
attributes and authenticated attributes are the only data types used in the CMS that require DER
encoding." Section 5.3 states the requirement; section 5.4 says why it binds a *verifier*, the
signature being over "the message digest of the complete DER encoding of the SignedAttrs value".
So the tolerance stays where the RFC writes BER, and `der::every_length_is_definite` is what a
caller holding a DER-only region asks. An indefinite length among the signed attributes is
`Authenticity::SignedAttributesNotDer` — a refusal by name, because digesting the file's octets
would answer *this signature does not verify* about a signature that may be good. The
`signedAttrs [0]` wrapper is outside the question on section 5.4's own authority: it replaces
those octets with an EXPLICIT SET OF tag, which this program already builds. Calibrated both
ways: `signed_attributes_that_are_not_der_are_refused_rather_than_digested` puts
two CMS values through `authenticity` differing by exactly the defect, and the control reaches the
arithmetic and answers `NotUnderThatKey`; `a_region_says_whether_every_length_in_it_is_definite`
moves the defect one level down, where a top-level check would miss it.

## 2. §12.8.1 — Table 256's `/DigestMethod` is a reader's entry, and is read
Table 256 is the **signature reference** dictionary, not the seed value dictionary, so §12.7.5.5's
"shall be used at the time the signature is applied" — session 1029's disposal of nineteen
seed-value entries — does not reach it. `Digest::from_pdf_name` reads the six names the table's
value list admits; `Signature::reference_digests` carries one answer per reference dictionary in
§12.8.2.1's order, in three states, because a name outside the list is a departure and an absent
entry is not. `viewer_core::notes` says both; nothing is acted on, §12.8.2.2.2's comparison not
being made, and the sentence says so.

## 3. §12.8.3.1/.3.3/.3.3.1 — two of the four curves, and five rows still `partial`
`bp256`/`bp384` 0.14.0 went stable 2026-09-10 and 09-11; ADR 0532 measured them as release
candidates on 08-23. Taken, on RFC 5639 (downloaded) sections 3.4 and 3.6, no new transitive
package. brainpoolP512r1 and Ed448 stay refused by name — `bp512` does not exist and
`ed448-goldilocks`'s only line with the scheme is 0.14.0-pre.15 (2026-09-14). ADR 1063 records
that the gap was always about *supply*, never about the specification. No `Judgement` says valid:
no host supplies an anchor (ADR 1039).
