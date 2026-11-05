# 2720 — A time-stamp trust condition is judged on the authority's own path, and a mandated revocation check per kind of certificate

Status: accepted. Session 1953. Context: ISO 32000-2 §12.8.3.4.4 ("… but shall enforce signature policy
constraints."); ETSI TR 102 272 clauses 6.6.2, 6.8 and 6.10; ETSI TS 119 172-2 clauses 4.23.2, 4.23.3 and 4.25.1,
which TS 119 172-3 adopts for its ASN.1 syntax; ETSI EN 319 122-1 clause 5.3; RFC 3161 section 2.3; RFC 5280
sections 5.3.2 and 6.1.3 (a)(3); ADRs 1067, 2714, 2715. The ETSI texts are cited and paraphrased (ADR 1085).

## One construction, two targets

ADR 2714 built a path from the signer's certificate to a policy's own trust points. A time-stamp trust condition
asks the same question of another certificate: the one that issued the signature's time-stamp token. So
`certification_path::search_authority` is the same search started from the token's signer over the certificates
the token carries, with RFC 3161 section 2.3's purpose (`id-kp-timeStamping`, whose extension a conforming
authority marks critical) at the instant the signer's path is asked about. Before the token's signer is anybody's
authority, the token must be the `signature-time-stamp` attribute clause 6.8 applies the condition to, its
imprint must be this signature's digest (EN 319 122-1 clause 5.3), and it must pass steps 1 to 3 of
`timestamp::signature_timestamp_established`, now shared as `timestamp::signature_timestamp_token`. Each failure
is its own `Found` variant and leaves the condition not evaluated, never broken. Where TR 102 272's condition
states no trees of its own, clause 6.8 applies the signing-certificate condition's trees; TS 119 172-2 clause
4.25.1 states no such rule, and there the condition is named with no search. Clause 6.10's
`tsaCertAlgorithmConstraints` is judged on the authority's certificate along that path, as `eeCert…` is on the
signer's. At most `MAX_AUTHORITY_SEARCHES` (4) conditions are searched for one signature: TS 119 172-2 allows
one per set of time evidences, and each asks about the same token.

## A revocation requirement is a kind of material, per kind of certificate

Clause 6.6.2 (and clause 4.23.2) mandates, separately for the end certificate and for the CA certificates, a
CRL check, an OCSP check, both, either, none, or `other`. `revocation::status` stops at the first answer of
either kind, so it cannot say which kind answered; `revocation::status_by` asks one kind alone, and
`policy_rules::revocation_checks` asks it of each certificate of each kind on the condition's path, below the
trust point, from §12.8.4's store and the signer's §12.8.3.3.2 attribute (`SignerFacts::with_material`). The
outcomes:

- **met**: no check mandated; no certificate of the kind below the trust point; or every certificate answered by
  the mandated material, none revoked at the instant;
- **broken**: the check found a certificate revoked with effect — the earlier of the revocation date and RFC 5280
  section 5.3.2's `invalidityDate` — at or before the instant, which section 6.1.3 (a)(3) makes a failed path;
  a revocation after the instant is an answer that the certificate was good then;
- **not evaluated**: no path; `other`, whose mechanism only its extension defines; or the file carries no
  material answering the mandated check for a certificate. The last is indeterminate rather than failed: a
  validator may fetch what a file lacks, this program answers revocation only from the file (ADR 1067).

Clause 4.23.3's freshness and retention members are named and not evaluated, so a requirement stating one is
never met as a whole.
