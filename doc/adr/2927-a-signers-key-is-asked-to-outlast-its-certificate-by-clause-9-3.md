# 2927 — A signer's key is asked to outlast its certificate, by clause 9.3

Session 2056, a signatures round (`doc/todo/77` item 3). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 2896 and 2905; ETSI TS 119 312 V2.1.1 (2026-06) clauses 3.4, 9.1 to 9.5; ETSI TS 119
172-4 V1.1.1 REQ-4.5-01 h). Cited by clause and paraphrased (ADR 1085).

## Context

REQ-4.5-01 h) asks for a suite's potential security issues against national rules or TS 119 312.
ADRs 2896 and 2905 asked TS 119 312's tables and clause 9.2's imprint rule, and left clauses 9.3 and
9.5. Clause 9.3 requires a signer's key to remain suitable over its certificate's validity period,
`notBefore` to `notAfter`; its notes say a key that does not can be protected by a time stamp made
with recommended mechanisms while the suite is still at least legacy. Clause 9.5 says the other keys
(a time-stamping unit's, a CA's, a CRL issuer's, an OCSP responder's) should resist over their own
certificate's validity and that of the certificates relying on it, shall be chosen at least as strong
as the keys they certify, and are under clause 9.4's trust-anchor rule.

## Decision

1. **Clause 9.3 is asked of the key's rows** — Table 2's algorithm, Table 3's curve, Tables 6 and 7's
   sizes — and not of Table 1's digest or Table 4's combination, which belong to one signature rather
   than to the key. The clause says *key*; the digest is clause 9.2's subject. A choice where the
   clause names no table.
2. **Suitable is clause 3.4's reading** (ADR 2896 reading 1): a recommended row stays suitable, a
   legacy row stops at the first instant after its phase-out day. A row whose phase-out has passed by
   the certificate's `notAfter` is `Issue::KeyOutlived`, `notAfter` included in the period as RFC
   5280 includes it. Classes only move from recommended towards legacy, so `notBefore` is not asked.
3. **Note 3's mitigation is not applied.** A note is not a requirement, and the issue is a
   *potential* one, which is what h) asks for; it is said whatever tokens cover the signature.
4. **An unlisted row is not asked again**: it already raised `NotListed`. A certificate whose
   validity did not read is `Issue::ValidityNotRead`, said rather than skipped.
5. **Clause 9.5 is not built.** Every key it names is outside what `applicability::Report` is
   handed: the path that validated is kept as its length only (`trust::Trust::Anchored`,
   `verdict::Valid`), the revocation issuers' and time-stamping units' certificates are not carried
   into the report. Building it means carrying the validated path's certificates, the material's
   signers and each token's signer into the report, and then choosing what "at least as strong"
   compares across families — clause 3.1 gives a security level for its classes (legacy at
   least 100 bits, recommended 125) and no per-key figure, so the comparison would be between classes.
6. **No national rules are held**, and none is claimed; h) names TS 119 312 as the one used.

## Consequences

A 2048-bit RSA signer certificate valid past 2026 raises Table 6's issue under clause 9.3 at any
validation instant, and a PKCS #1 v1.5 one valid past 2034 raises Table 2's. Dropping the call from
`Report::of` fails the report's own test. Left of item 3: clause 9.5, and national rules.
