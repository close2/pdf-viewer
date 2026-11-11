# 2797 — A certified signer attribute is judged by RFC 5755 section 5 under the policy's trust trees

Status: accepted. Session 1993. Context: ADRs 1085, 2714, 2720, 2725, 2786, 2787; RFC 5755 sections 4.1, 4.2, 4.3.2,
4.5 and 5 and Appendix B; ETSI TR 102 272 clause 6.9; ETSI TS 119 172-2 clause 4.26.1; ETSI TS 119 172-3 clause
4.26. The ETSI texts are cited and paraphrased (ADR 1085).

## Why this slice

ADR 2787 left the certified half of the attribute residue first in line, headed by an RFC 5755 reader. A reader that
nothing judges would be a placeholder, so the reader and section 5's validation are built together, and both
syntaxes' judgements read them: TR 102 272's attribute trust condition and TS 119 172's sets. An attribute
certificate's own revocation, signed assertions and `aaCertAlgorithmConstraints` are each their own construction,
and are left named.

## The reader

`attribute_certificate::read` reads section 4.1 under Appendix B's `IMPLICIT TAGS`: `baseCertificateID [0]`,
`entityName [1]` and `v2Form [0]` replace their types' tags, and only `directoryName [4]` wraps its `Name`, because
X.680 makes a tag on a `CHOICE` explicit. What section 4.2 requires is refused as malformed: a version other than v2,
an issuer other than the `v2Form` holding one non-empty `directoryName`, an empty attribute list or a type repeated.
Attributes are bounded by `MAX_ATTRIBUTES`, extensions and names by `MAX_ENTRIES`; past a bound the certificate is
undecided, not invalid.

## Section 5, condition by condition

- **1, the holder.** The holder is the signer: a `baseCertificateID` names the certificate of that issuer and serial
  number, and an `entityName` the subject or one of the subject's alternative names (section 4.2.2). A
  `directoryName` is compared as the path code compares names, and another form only by identical encoding, since
  each has its own matching rule. Options that disagree, an `issuerUID`, or an `objectDigestInfo` are undecided. The
  holder's path is the signer's, which the signing-certificate condition already searched; its verdict is reused.
- **2 and 4, the issuer.** The issuer's certificate is a carried one whose subject is the issuer's name and whose key
  verifies the signature over `acinfo`. Condition 4 asks for an issuer trusted "by configuration or otherwise", and a
  policy is that configuration: clause 6.9 makes `attrCertificateTrustTrees` the path conditions for any attribute
  certificate, the signing certificate's where it states none, and clause 4.26.1 item 2 gives a set its own trees.
  So the issuer's path is searched to them with `certification_path::search_certificate`. TS 119 172 states no
  fallback for a set without trees, so there the certificate is undecided.
- **3, section 4.5.** The issuer's certificate may not assert `cA`, and a key usage that leaves out
  `digitalSignature` refuses signatures.
- **5, the instant.** It is the instant the policy's paths are validated at, with both ends of the period included.
- **6, targeting.** A signature's verifier names no server or group, so whether it matches a target is undecided.
- **7, critical extensions.** Those of section 4.3 that state no condition on validity (audit identity, authority
  key identifier, information access, distribution points, `noRevAvail`) are supported; any other critical one
  rejects the certificate.

Any failed condition decides **invalid**. Otherwise any undecided one decides **undecided**, as does a revocation
requirement, because the certificate's own revocation is not yet checked. Only otherwise is it **valid**.

## What the outcome decides

Each attribute certificate of an admitted kind gets a finding: met where it is valid, broken where it is invalid, not
evaluated where it is undecided. TR 102 272 clause 6.9 states the path conditions for every attribute certificate, and
clause 4.26.1 holds every signer attribute a set applies to to its requirements, so an invalid one breaks the
condition. A set naming types applies only to a certificate carrying one of them, and to one that does not read, which
might, it gives not evaluated. ADR 2787's reading of kinds stands: a certificate of a kind the condition does not
admit is passed over by clause 6.9. Under a TS 119 172 set it is read for its types, and it breaks the level only
where it carries a type the set applies to.

A certificate certifies an attribute only where it is valid. So `attributeMandated` and a set's mandated type are met
by a valid certificate carrying the type. They are not evaluated where an undecided one might carry it, and broken
where neither a claim nor any certificate could.

## Left

An attribute certificate's own revocation (`attrRevReq`, `AttributeRevocationTrust`) is left. So are signed
assertions, clause 6.10's `aaCertAlgorithmConstraints`, value constraints and trusted-list anchors. Nothing here is
owner-blocked.
