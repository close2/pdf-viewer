# 1953 — A time-stamp trust condition is judged on the authority's own path

Session 1953, a single ledger round (§12.8.3.4.4), the ledger lane. ADRs 2720 and 2721; no question.

**The premise held.** TR 102 272 clause 6.8 and TS 119 172-2 clause 4.25.1 item 3 (TS 119 172-3 Annex A's
`TimestampTrustCondition`) were parsed as a name only (TR) or a `trust_stated` flag (both TS syntaxes), and the
revocation requirement of either condition collapsed to `revocation_mandated`; `revocation::status` stops at the
first answer of either kind. No corpus signature states a machine-readable policy (ADR 1709): every policy in the
tests is constructed, over a hierarchy minted for them (root, intermediate, time-stamping authority, a token over
the CMS fixtures' signature field, each issuer's CRL, one revoking the authority, and an OCSP response).

**Built (ADR 2720).** `certification_path::search_authority`: the search of ADR 2714 from the token's signer, with
`id-kp-timeStamping`, only for the `signature-time-stamp` token whose imprint is this signature's and which passes
steps 1 to 3 (now `timestamp::signature_timestamp_token`, shared); TR's condition inherits the signer's trees where
it states none (clause 6.8); `Authorities` bounds one signature at four searches. `revocation::status_by` asks one
kind of material; `policy_rules::revocation_checks` answers clause 6.6.2's / 4.23.2's check per kind (end
certificate, CA certificates below the trust point): met, broken where revoked with effect by the instant (the
earlier of revocation and `invalidityDate`), not evaluated where unanswered or `other`. Clause 6.10's
`tsaCertAlgorithmConstraints` is judged on the authority's certificate. `SignerFacts` carries the material: the
signer's §12.8.3.3.2 attribute, and §12.8.4's store through `with_material`, which `viewer-core` passes.

**Tests**: seven in `revocation_checks/tests.rs` (authority path met and another root; no token, another
signature's token, an unbound token; the search bound; CRL per kind; kind of material for all six checks; revoked
before and after the instant; no path and no CA) and one end-to-end per syntax. `viewer-host`'s enforced-copy
test counts two revocation constraints where it counted one. **Plant**: six defects (revocation never breaks;
`status_by` ignores the kind; no time-stamping purpose; imprint not compared; CA check includes the trust point;
clause 6.8's inheritance dropped) fail 2, 2, 9, 1, 4 and, after a test was added for it, 1.

**Gates.** fmt `--check` (`pdf-signature`, `viewer-core`, `viewer-host`): exit 0. clippy `-D warnings
--all-targets` (the three): exit 0. nextest: `pdf-signature` 331 of 331, `viewer-core` 443 of 443, `viewer-host --test
signature_policy` 11 of 11. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh --round 1953 docs`:
exit 0. Tier 2 not run: the change reaches `policy_rules`, `revocation`'s new entry and `timestamp`'s shared steps,
which only `viewer-core` and `viewer-host` call, and no tier-2 line enforces a policy.

**Left of the row** (ADR 2721): the policy tree and name constraints (trust points and the authority), attribute
certificates, trusted lists, clause 4.23.3's freshness and retention, clause 6.8's caution period and delay (work);
`other` revocation mechanisms and human-readable policies (limits); a C caller's fetched copy (host lane).
