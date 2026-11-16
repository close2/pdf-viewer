# 2895 — A signer attribute's value is compared by its type's own equality rule, and a signed assertion's silence is recorded

Session 2039, a signatures round. Status: implemented. Rows: §12.8.3.4.4. Context: ADRs 1085,
2782, 2786, 2787, 2797; ETSI TR 102 272 V1.1.1 clause 6.9; ETSI TS 119 172-2 V1.1.1 clause 4.26;
TS 119 172-3 V1.1.1 clause 4.26 and Annex A; ETSI EN 319 122-1 V1.3.1 clauses 5.2.6.1 to 5.2.6.3;
ETSI EN 319 102-1 V1.4.1 clause 5.2.8.4.2.7; ITU-T X.501 (10/2019) clauses 8.9.2.1, 13.4.2 and
13.4.7; ITU-T X.520 (10/2019) clauses 6, 7 and 8.1.1; RFC 5280 section 7.1. The ETSI texts are
cited and paraphrased (ADR 1085).

## Value constraints

**TR 102 272 clause 6.9.** `attributeValueConstraints` is a `SEQUENCE OF AttributeTypeAndValue`.
The values it lists are the ones the policy considers valid. A value is one of them where X.501
clause 8.9.2.1's evaluation of an attribute value assertion is true. That evaluation is undefined
where the type is unknown, has no equality matching rule, or the value does not conform to the
rule's syntax; true where the type's equality matching rule matches; false otherwise. Clause 13.4.7
says a type without an equality rule has its values never compared, and clause 13.4.2 that a
subtype keeps its supertype's rules.

- `signer_attributes::values` holds the rule for the X.520 `id-at` types whose definitions state
  `caseIgnoreMatch`, or that are subtypes of `name`, over a `DirectoryString` syntax: 26 types,
  listed in `CASE_IGNORE`. X.520 clause 8.1.1 makes two values equal when clause 7's preparation
  makes the same string of them. On ASCII the preparation is exact: controls mapped, case folded,
  empty prohibited, spaces trimmed and collapsed (clause 7.6.1, with its example as a test). Any
  other character needs the Unicode tables, which are not in this tree, so the comparison is
  undecided, as `trust::names` already decides for RFC 5280 section 7.1. `TeletexString` is
  declined under clause 7.1.
- **An attribute is within by value where every value it carries matches a listed value of its
  type.** The clause lists values that "are considered valid". An attribute carrying a value
  outside them carries something the policy does not consider valid, so one listed value cannot
  admit it. Any value matching nothing makes the attribute not within. Any undefined comparison,
  with none failing, makes it undecided.
- **Types left open, written down.** `id-at-role` (2.5.4.72) is defined in ITU-T X.509, which
  this tree does not hold, so its rule is not known here. `dc` states `caseIgnoreMatch` over
  `IA5String`, which is not that rule's syntax, so clause 8.9.2.1 (a)(3) makes every comparison
  undefined. Every other type (bit strings, telephone numbers, object identifiers, distinguished
  names and the rest) has a rule this round did not build. Each is undecided and named by
  `Check::ConditionValues`, never met and never broken.
- The claimed attributes' and the attribute certificates' `SET OF` values are now read beside
  their types (`Carried::claimed_values`, `AttributeCertificate::values`). A claimed attribute
  without its values is unreadable, as its syntax requires them.

**TS 119 172.** Clause 4.26 item 4 allows a constraint on a value. TS 119 172-3's
`attributeValueConstraints` is a sequence of `Other` from `AttributeValueConstraintsOtherSet`,
whose clause note says no identifier is defined for it. TS 119 172-2's `AttributeValueConstraint`
is `AnyType`. Neither syntax gives a constraint a form. This is a silence of the standard, not
work: `Check::Values` stays not evaluated, and its sentence says so.

## Signed assertions

EN 319 122-1 clause 5.2.6.1 defines `signedAssertions` as a non-empty sequence of
`SignedAssertion ::= SEQUENCE { signedAssertionID, signedAssertion OPTIONAL }`. Its information
object class is open, so any identifier may appear. The text says only that each assertion "shall
be signed by third party". Clause 5.2.6.3 defines one form, `id-ets-signedSAML`, an `OCTET STRING`
holding a signed SAML assertion, and calls it an example. TS 119 172-2 clause 4.26 lets a policy
accept the level `SignedAssertion` and state trust trees and revocation trust for it (items 2 and
3), and require an attribute identifier (item 4). EN 319 102-1 clause 5.2.8.4.2.7 then requires a
validator to validate the signed assertions "following the rules established in ISO/IEC 9594-8"
and check them against the constraints.

So:

- **For an identifier other than `id-ets-signedSAML`, the texts define nothing**: no syntax for
  the value, no signature to verify, no attribute to find. That is a limit.
- **For the SAML form**, the judgement exists, but it needs a SAML 2.0 assertion reader (OASIS
  SAML core: the assertion, its attribute statements, its enveloped XML Signature), the signer's
  path to the item 2 trees, and the item 3 checks. X.509 has no rule for a SAML assertion's
  attributes, so the reading of `AttributeIdMustBePresent` against a SAML attribute's `Name` would
  need its own ADR. That is work; no owner's word is needed. This round did not take it, because
  it is a reader of its own and did not fit the wall budget beside the two items above.
- `Check::Assertions` stays not evaluated, as it was, and §12.8.3.4.4's note names both halves.
