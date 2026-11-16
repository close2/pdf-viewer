# 2037 — An attribute certificate's chain is checked, and its authority's algorithm

Session 2037, a ledger round, the signatures lane (§12.8.3.4.4). ADRs 2890 and 2891; no question.

**The slice, and why.** ADR 2782's table was stale. The RFC 5755 reader, its fuzzing through
`fuzz/`'s `cms` target, section 5's validation (ADR 2797) and the OCSP archive cutoff for retention
(ADR 2785) were already built. What the row still named as work was the certificate's own revocation
under `attrRevReq` or `AttributeRevocationTrust`, and clause 6.10's `aaCertAlgorithmConstraints`.
Both concern the attribute authority, so this round took them together. That completes the certified
attribute's judgement, apart from targeting.

**The premise.** Effort only. No `A*` after A800 cites either, so no Q864 was asked.

**Built.** `revocation::Subject` names a certificate by issuer and serial, so the same check covers
both kinds. `revocation_checks::attribute_checks` covers the attribute certificate and its
authority's certificate (end entities) and the authority path's CA certificates. A revoked
certificate makes the attribute certificate invalid; an unanswered check leaves it undecided, with
`noRevAvail` named. The requirement is read whole in all three syntaxes. A set's retention is named
as not evaluated. `aaCertAlgorithmConstraints` and `Usage#AaCert` are judged per carried certificate
(`certified::made`), the algorithm being the one TS 119 172-2 clause 4.29.1 names for "generating
attribute certificates". `policy_rules`' module doc and `doc/todo/65` §5 corrected.

**Tests.** Six in `certified/tests/revocation_tests.rs`, over a CA, an authority and two attribute
certificates, with five CRLs and three OCSP responses that `r2037/gen/{make,material}.sh` minted
from keys this tree made. Two more in each syntax's tests. **Plants**: skip the attribute
certificate's own check, admit an unlisted algorithm, and drop the `AaCert` judgement. Six tests
failed; reversed, all pass.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-signature`): 0. nextest `pdf-signature`:
595 of 595, 5 skipped. `fuzz/` `cargo check --bins`: 0 (the `revocation` target updated).
Dependents' `cargo check` (viewer-core, -ffi, -host, pdf-archive, pdf-transform): 0. Conformance
after `git add -N`: 474 passed. Tier 2: `corpus` 101, 974 documents, 61 incomplete as at HEAD.
`raster_golden` 101, held 951, moved 23: the same 23 names as round 2035's. No drawing path was
reached. `state.sh --round 2037 docs`: 0.

**Habit, and what is left.** When a comment in a clause's syntax is too short to decide a member,
read the successor document that carries the member forward. TS 119 172-2's usage list decided what
TR 102 272's `aaCert…` constrains. The same list reads `tsaCert…` as the authority's tokens, and ADR
2720 reads it as the authority's certificate; that is reported for a later round. §12.8.3.4.4 stays
`partial`. Left: signed assertions, value constraints, unsupplied lists.
