# 2714 — A signature policy's trust condition is evaluated on a path to the policy's own trust points

Status: accepted. Session 1950. Context: ISO 32000-2 §12.8.3.4.4 ("… but shall enforce signature policy
constraints."); ETSI TR 102 272 clauses 6.5.1, 6.6.1, 6.7 and 6.10; ETSI TS 119 172-2 clauses 4.22.2, 4.22.5,
4.24 and 4.29, which TS 119 172-3 adopts for its ASN.1 syntax; ETSI EN 319 102-1 clause 5.2.6.4; RFC 5280
section 6.1; ADRs 1076, 2560, 2580, 2604, 2711. The ETSI texts are cited and paraphrased (ADR 1085).

## The two sets of anchors are two questions

ADR 1076 makes the anchors of §12.8.1's third question a host's input: whom the reader believes. A policy's
signing-certificate trust condition names trust points of its own: the hierarchies the agreement admits. Both are
asked, separately: `trust::validate_path` builds one path to the host's anchors for the verdict, and
`policy_rules::certification_path::search` builds another to each of the policy's trust points, one point at a
time, over the certificates the `SignedData` carries and nothing else. The policy's answer can withhold the word
*valid* (`Verdict::under_policy`) and never grant it, so a policy cannot make a signature valid by naming its own
root, and a host's anchor cannot make a policy's condition met.

## The outcome, by EN 319 102-1 clause 5.2.6.4

- **Met**: a path validates under RFC 5280 section 6.1 to a trust point, and holds no more CA certificates below
  it than the point's `pathLenConstraint` (zero admits the trust point and an end-entity certificate only).
- **Broken**: a path validated but is longer than that constraint, or every chain to a trust point was refused by
  RFC 5280 for a defect of the path (not current, a signature that does not verify, not a CA, a key usage) —
  and no anchor was left unsearched.
- **Not evaluated**: no chain at all (step 2 a) calls it indeterminate, not failed); the only trust point reached
  is past its `reliableUntil` (steps 1 a) and 3)); the refusal is this program's limit (an algorithm it does not
  verify, an unrecognised critical extension); the trust point also states an acceptable policy set, name
  constraints, policy constraints or a mandated path other than `asInSignature`, which `trust` does not process;
  or an anchor is a trusted-list reference, which this program does not fetch, or a trust point past
  `MAX_TRUST_POINTS` (16). An unsearched anchor might have validated another path, so it keeps the condition from
  being called broken.

## The instant

RFC 5280 section 6.1.1 (b) is the host's: where a host supplied anchors, `viewer-core` passes the instant its
verdict's path is asked about (`SignerFacts::validated_at`), and otherwise a covering signature timestamp's
instant is used, the one the signing period is already judged at. With neither, the condition says it had none.

## What reads the path

- **`fullPath` certificate information** (TR 102 272 clause 6.5.1, TS 119 172-2 clause 4.24.3) is met by any path
  that validated, since it is built from the carried certificates alone, and not evaluated without one.
- **`fullPath` certificate reference** (TR 102 272 clause 6.5.1) asks one signing-certificate attribute to hold a
  hash of every certificate below the trust point. RFC 5035 section 5.4 evaluates the two attributes
  independently, so either one may meet it; `ess::certificate_hashes` reads every identifier, bounded at
  `MAX_IDENTIFIERS`. "Up to a trust point" is read as excluding the trust point, which the policy itself holds.
  That is a reading, and the sentence names it.
- **Clause 6.10's `eeCertAlgorithmConstraints`** judge the signer's certificate (its algorithm, and the issuing
  key's length); **`caCertAlgorithmConstraints`** each CA certificate below the trust point. TS 119 172-2 clause
  4.29's signing-certificate usage now measures the issuing key, and its path-certificate usage
  (`Usage#PathCert`) judges the same CA certificates. A path with no CA certificate below the trust point meets
  the CA lists vacuously, and says so.

## Not decided here

The revocation checks a condition mandates (clause 6.6.2, clause 4.23), the time-stamp and attribute trust
conditions, the attribute- and time-stamping-authority lists and the policy tree are named and not evaluated;
ADR 2715 says why each stays and what the row's status is.
