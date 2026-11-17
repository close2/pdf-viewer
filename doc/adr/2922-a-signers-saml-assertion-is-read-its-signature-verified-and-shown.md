# 2922 — A signer's SAML assertion is read, its signature verified by SAML's own profile, and shown

Session 2053, a signatures round. Status: implemented. Rows: §12.8.3.4.4. Context: ADRs 1085,
2604, 2843, 2895; ETSI EN 319 122-1 V1.3.1 clauses 5.2.6.1 to 5.2.6.3; ETSI EN 319 102-1 V1.4.1
clause 5.2.8.4.2.7; ETSI TS 119 172-2 V1.1.1 clause 4.26; OASIS SAML 2.0 core (`saml-core-2.0-os`)
sections 1.3.3, 1.3.4, 2.3.3, 2.5.1, 2.7.3 and 5.4; XML Signature 1.1 section 4.5.4. The ETSI texts
are cited and paraphrased (ADR 1085); SAML core was fetched from OASIS and is not tracked.

## What the texts owe a verifier

EN 319 122-1 clause 5.2.6.2 lets a claimed attribute be a SAML assertion (`id-aa-ets-claimedSAML`,
an `OCTET STRING`), and clause 5.2.6.3 defines one form of signed assertion, `id-ets-signedSAML`,
the octets of a signed SAML assertion. EN 319 102-1 clause 5.2.8.4.2.7 then splits in two:

- **Where no rule constrains signed assertions**, the validator makes their value available to the
  driving application. That is a display: the report's REQ-4.5-01 f) item, which already lists every
  attribute of the signer, now carries the `signer-attributes-v2`'s assertions with it.
- **Where a policy's set constrains them**, each is validated and checked against the constraints,
  and a failure is the constraints' failure (`asserted.rs`, below).

**Does the clause require the assertion's own XML signature to be verified?** Yes, and not by
inference: SAML core section 2.3.3 says that where an assertion is signed, a relying party verifies
the signature by XML Signature and does not rely on the contents of one whose signature is invalid.
Showing an assertion is relying on it, so the signature is verified on both routes. The work was
already in the tree: `xml_signature` held XML Signature's core validation and exclusive
canonicalisation for a trusted list (ADR 2843), so SAML's profile is a second entry point,
`verify_saml`, not a second implementation. No dependency was added; `xmlparser` and `xml.rs`'s
bounded tree read the assertion as they read a policy and a list (ADR 2604).

## SAML core section 5.4, as built

- One `ds:Signature` among the assertion's children, enveloped (5.4.1); exactly one reference, to
  `#` and the root's `ID` (5.4.2), resolved through `ID` (section 1.3.4) where a list resolves `Id`.
- A transform other than the enveloped transform and exclusive canonicalisation is rejected, as
  5.4.4 permits: the alternative is proving that no content was left out.
- `ds:KeyInfo` may be absent (5.4.5). Then no key is known and the signature is **unverified**,
  said as such and never as valid. XML Signature 1.1 section 4.5.4 relates every certificate of an
  `X509Data` to the validation key, so the signer is the carried certificate whose key verifies.
- Failure is split: a digest or value that does not match, or a shape 5.4 forbids, is invalid, and
  the assertion's attributes are withheld from the report and from any judgement; a missing key or
  an algorithm this program does not compute is unverified, and the attributes are shown as read.

## A constrained assertion

`policy_rules::signer_attributes::asserted` judges each signed assertion of a set that accepts them:
it reads; it is signed (clause 5.2.6.1 requires a third party's signature); the signature verifies;
the verifying certificate's path validates to the set's trust trees (TS 119 172-2 clause 4.26 item
2, by X.509's rules as clause 5.2.8.4.2.7 item 1 says); section 2.5.1's conditions hold at the
instant evaluated at, the audience restriction indeterminate because no audience names a
signature's verifier; and the set's revocation requirement (item 3) is answered over that path. The
first failure breaks the set, else the first undecided leaves it not evaluated. Neither SAML core nor
EN 319 122-1 relates an assertion's subject to the signer, so the subject is shown and never
compared, and the sentence of a valid assertion says so.

## Bounds

64 KiB of octets, 4 096 elements and declarations, 64 attributes of 64 values each; a document type
declaration is refused by `xml.rs` before any entity could expand.
