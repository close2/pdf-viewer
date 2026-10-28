# 2560 — An ASN.1 signature policy bound to its signature is enforced constraint by constraint, and what it cannot evaluate is named

Status: accepted. Session 1863, a clause round of batch one hundred and twenty-seven, on the owner's answer A713.
Context: ISO 32000-2 §12.8.3.4.4 ("During validation conforming signature handlers should not enforce seed values
constraints, if present, but shall enforce signature policy constraints."); ADRs 1076, 1085, 1219, 1709, 1728, 2529;
ETSI TR 102 272 clauses 6.1 to 6.11 and Annex B.11, ETSI TS 119 172-3 clauses 4.2 to 4.4, ETSI EN 319 122-1 clauses
5.2.9.1 and 5.2.10 — every ETSI text cited by clause and never quoted (their notice forbids reproduction).

## Decision

`pdf_signature::policy_rules` reads ETSI TR 102 272 clause 6's `SignaturePolicy` (the syntax RFC 3125 also publishes)
out of clause 5.2.10's stored policy document and judges a signature against it. Each constraint is one `Finding` —
met, broken, or not evaluated — and `viewer-core`'s notes say each as a sentence beside a count of the broken ones.
**A broken constraint withholds the verdict**: `verdict::Verdict::under_policy` turns a `Valid` into
`Reservation::PolicyConstraintBroken` naming the first broken finding, and changes nothing else, so a policy can withhold
the word and never grant it; a constraint not evaluated withholds nothing and is said as such.

**When a document is enforced at all.** Only when it is bound to the signature and names the signer's policy: its
octets digest to the value the signer signed (`policy::Binding::Matches`), or the signer signed the policy's own hash —
TR 102 272 Annex B.11 gives that as the second way a signer obtains the value, and clause 6.1 requires the own hash to
be re-computed over the policy information without its outer type and length, which the parser does and refuses on
disagreement. The store is unsigned (EN 319 122-1 clause 5.2.10), so an unbound document is nobody's agreement, and a
zero hash (clause 5.2.9.1's *not known*) binds nothing either.

**Enforced**: clause 6.2's signing period against the signature timestamp's `genTime` where its imprint covers this
signature, and not evaluated where nothing establishes the time; clause 6.5.1's mandated signed and unsigned attributes,
`externalSignedData`, the `signerOnly` certificate reference (a signing-certificate attribute) and certificate
information (the signer's certificate carried); clause 6.4's commitment-type selection; clause 6.10's
`signerAlgorithmConstraints`. **Named, not evaluated**: the trust conditions of clauses 6.7 to 6.9 (their trust points
are not the host's anchors of ADR 1076), clause 6.10's four certificate-issuer lists, the `fullPath` requirements (the
path is the anchors' question), clause 6.5.2's verifier rules (they ask a verifier to add attributes, which this reader
never does), and every clause 6.11 extension.

## The readings, each a choice a later round must not re-litigate

- **The signing period is enforced although clause 6.2 states it with a recommendation's modal.** The modal is addressed
  to signers; §12.8.3.4.4
  makes enforcing the policy's constraints a validator's `shall`, and the period is one of them.
- **Clause 6.10 is read literally**: the list identifies the algorithms that may be used, so both the `SignerInfo`'s
  digest algorithm and its signature algorithm must be listed, and `minKeyLength` is compared with the signer's key
  (RSA and DSA modulus, EC curve size) for the signature algorithm only. An EdDSA key's length is not measured and the
  finding says so.
- **Clause 6.3's placement rules refuse the document.** A rule in both the common rules and a commitment rule, or one of
  the three required rules in neither, is a malformed policy; with no commitment rule at all, *every commitment rule*
  holds vacuously, which is the text's own logic.
- **TS 119 172-3's ASN.1 is told apart, not misread**: its top level opens with a `Digest` (clause 4.2.2), whose first
  member is a sequence where TR 102 272's `AlgorithmIdentifier` opens with an object identifier, and it is refused by
  name as another syntax.

## Budgets

The document is hostile until bound: 256 KiB at most, nesting held at `der::MAX_DEPTH` by `der::is_canonical`'s
pre-pass — which also enforces clause 6.1's DER and walks the whole document before any list is allocated — and 4096
list entries across the document, each charged before it is pushed. Tests sit at each bound.

## What stays owed, so §12.8.3.4.4 stays `partial`

TS 119 172-3's own rules (its Annex A module, a different vocabulary of constraints), TS 119 172-2's XML syntax,
human-readable policies (prose no validator enforces), and enforcement over a copy fetched at the URL qualifier
(`viewer_host::policy` still says the constraints were not enforced). ADR 2561 and Q729 are unused.
