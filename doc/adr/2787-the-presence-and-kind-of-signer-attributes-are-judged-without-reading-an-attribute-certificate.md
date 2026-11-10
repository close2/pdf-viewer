# 2787 — The presence and kind of signer attributes are judged without reading an attribute certificate

Status: accepted. Session 1988. Context: ADRs 1085, 2580, 2604, 2725, 2782, 2786; ETSI TR 102 272 clause 6.9; ETSI
TS 119 172-2 clause 4.26; ETSI TS 119 172-3 clause 4.26; ETSI EN 319 122-1 clause 5.2.6.1 and Annex A.2.3; RFC 5126
section 5.11.3. The ETSI texts are cited and paraphrased (ADR 1085).

## What a signature carries

EN 319 122-1 clause 5.2.6.1's `signer-attributes-v2` and RFC 5126 section 5.11.3's `signer-attributes` (deprecated
by Annex A.2.3, not forbidden to a validator) are both signed attributes, and both modules are `EXPLICIT TAGS`.
`policy_rules::signer_attributes::carried` reads the signed ones together into the type of each claimed attribute and
a count of attribute certificates and signed assertions, under `MAX_SIGNER_ATTRIBUTES` entries. A value of another
shape, an attribute with other than one value, or a truncated attribute list is named unreadable, and every finding
over it is not evaluated: a part read is not the whole the clause speaks of.

## The readings each judgement rests on

- **TR 102 272 clause 6.9.** `attributeMandated` true requires an attribute within the condition's constraints. A
  claimed attribute is within them where `howCertAttribute` admits claims and its type is in
  `attributeTypeConstraints`, or neither constraint list is stated; one whose type only `attributeValueConstraints`
  names depends on its value, so it is undecided. The clause says which attributes are *considered*, not that another
  one breaks the signature, so an attribute of a kind not admitted is passed over. A certified one of an admitted kind
  leaves the requirement not evaluated, and is itself named not evaluated.
- **TS 119 172-2 and -3 clause 4.26.** Each identified attribute must be carried, and every signer attribute must meet
  the set's level of assurance. A set naming no identifier applies its level to every signer attribute, so the clause's
  last paragraph makes a set that names some apply it to those. Every set in the list applies, so each is judged on its
  own. A claimed attribute's type is compared, so a claimed attribute below the level, or a mandated type nobody
  claims, is **broken**. An attribute certificate or a signed assertion is never opened: below the level under a set
  naming nothing it is broken, and where the set names types it might carry one, so it is not evaluated; of an
  accepted kind it may carry a mandated type, so that requirement is not evaluated where no claim meets it.
- **Values are not compared.** X.501 makes two values of one attribute type equal by that type's matching rule, and
  this program holds none per type, so every value constraint is named not evaluated. That is work, not a limit.
- **Syntax.** TS 119 172-2 clause 4.26.1 forbids value constraints without an identifier, and TS 119 172-3 adopts
  its semantics, so the ASN.1 reader refuses that. TS 119 172-2 writes an identifier that is an object identifier as
  an RFC 3061 URN, which is read as the object identifier it spells.

## Left, in the order a round takes it

The certified half from ADR 2786 is next: an RFC 5755 reader, the attribute authority's signature and path to the
condition's trust trees (the signer's where none are stated), the holder bound to the signer's certificate, validity,
and revocation by `attrRevReq` or `AttributeRevocationTrust`. Then the types an attribute certificate carries, which
decide the not-evaluated cases above, and clause 6.10's `aaCertAlgorithmConstraints`. Value matching stays named until
a matching rule is held per type. Nothing here is owner-blocked.

## The defect this found in §12.8.3.4.3 (j)

The emptiness check of clause 5.2.6.1 read `certifiedAttributesV2` and `signedAssertions` as if implicitly tagged. So
an empty list inside its explicit tag was missed, and the test fixture was implicitly tagged too. Both now follow the
module's `EXPLICIT TAGS`.
