# 1950 — A signature policy's trust condition is evaluated on a path to its own trust points

Session 1950, a single ledger round (§12.8.3.4.4), the ledger lane. ADRs 2714 and 2715; no question.

**The premise held for the work and not for its numbering.** The trust conditions of clauses 6.7 to 6.9
are TR 102 272's (RFC 3125's) numbering, not TS 119 172-3's, whose equivalents are clauses 4.22 to 4.24
with semantics in TS 119 172-2; the row note said "TS 119 172-3 clauses 6.7 to 6.9". All three
syntaxes parsed the trust trees as a name only (`trust_conditions`, `SigningCertificate(info)`, an
XML element required and skipped), and `trust` returned a path's length, never the path. No corpus
signature states a machine-readable policy (ADR 1709), so every policy in the tests is constructed.

**Built (ADR 2714).** `policy_rules::certification_path`: trust trees with certificate trust points
(`pathLenConstraint`, TS 119 172-2's `reliableUntil`, unprocessed members named) and trusted-list
anchors counted; `search` builds a path from the signer over the carried certificates to each point
(at most 16) with `trust::validate_path`, new, which returns the validated path; outcomes by EN 319
102-1 clause 5.2.6.4 (no chain or a past sunset indeterminate, a refused or over-long path broken, an
unsearched anchor never broken). Read off the path: `fullPath` reference (`ess::certificate_hashes`,
bounded) and information, clause 6.10's ee and CA issuer lists, clause 4.29's signing-certificate key
length and `Usage#PathCert`. Revocation requirements are parsed and named; `noCheck` is met.
`SignerFacts::validated_at` carries the host's instant; `viewer-core` passes the verdict's.

**Tests**: ten unit tests over `trust`'s fixture hierarchy (met; length 0 broken and 1 met; no chain;
not current, and beside a trusted list; sunset and its control; unprocessed member; no instant and the
timestamp fallback; the 16-point bound; full-path reference) and one end-to-end test per syntax.
**Plant**: Longer met, unprocessed ignored, reference over the signer only: 4, 2 and 1 of 81 fail; a
fourth (authorities including the trust point) is equivalent, `issued` already drops the last.

**Gates.** fmt `--check` (`pdf-signature`, `viewer-core`, `viewer-host`): exit 0. clippy `-D warnings
--all-targets` (the three): exit 0. nextest: `pdf-signature` 321 of 321, `viewer-core` 443 of 443,
`viewer-host --test signature_policy` 11 of 11. `cargo test -p conformance` after `git add -N`: exit 0.
`state.sh --round 1950 docs`: exit 0. Tier 2 not run: rule 2 owes all of it, but the change reaches
`policy_rules`, `ess` and `trust::validate_for`'s delegation, which only `viewer-core` and
`viewer-host` call, and no tier-2 line enforces a policy or validates a path.

**Left of the row** (ADR 2715): the policy tree and name constraints, revocation checks by kind, the
time-stamp and attribute conditions with their issuer lists, trusted lists (work); human-readable
policies (a limit); a C caller's fetched copy (host lane).
