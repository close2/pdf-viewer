# 1993 — A certified signer attribute is judged by RFC 5755 section 5, and an acceptable policy set binds

Session 1993, a single ledger round (§12.8.3.4.4), the second ledger lane. ADRs 2797 and 2798; no question.

**First, A796.** The orchestrator added the owner's answer to Q796 (option 2), which no round had built, ahead of the
slice. `certification_path::implied` gives a trust point that states an acceptable policy set
`initial-explicit-policy` (RFC 5280 section 6.1.1 (f), `requireExplicitPolicy` 0). Every sentence names the set as
one this program reads as requiring the path to be valid for one of its policies (ADR 2798).

**The slice, and why.** ADR 2787 left the certified half next. A reader that nothing judges is a placeholder, so the
RFC 5755 reader and section 5's seven conditions were built together, judged in TR 102 272 and in both TS 119 172
syntaxes. An attribute certificate's own revocation, signed assertions and `aaCertAlgorithmConstraints` are each a
construction of their own, and are left named.

**Premise.** It held. `carried` counted attribute certificates and opened none, both conditions recorded only
whether trust trees were stated, and every certificate was not evaluated. RFC 5755 and the two ETSI clauses decide
every case built, and `certification_path`'s search needed only a target other than the signer's certificate.

**Built.** `attribute_certificate::read` reads section 4.1 under Appendix B's `IMPLICIT TAGS`, refuses section 4.2's
violations, and bounds attributes and names. `signer_attributes::certified::judge` checks the holder (by
`baseCertificateID` or `entityName`, plus the signer's own path verdict), the issuer's signature and section 4.5
profile, the issuer's path through `search_certificate`, validity, targeting and critical extensions. Its outcome is
valid, invalid or undecided, and a revocation requirement leaves it undecided. TR 102 272 falls back to the signing
certificate's trees, as clause 6.9 directs. Each certificate of an admitted kind is a finding, and only a valid one
certifies its types; TS 119 172's level now reads a refused-kind certificate's types. Both parsers keep the trees.
The `cms` fuzz target reads every carried attribute certificate.

**Tests.** `attribute_certificate/tests.rs` (three: the signed fixtures read, the profile's violations and the
implicit tagging, the bound), `certified/tests.rs` (three: valid, each failed condition, each undecided one),
`signer_attributes/tests.rs` (two new, end to end in both syntaxes, three updated where a certificate's types now
decide) and `certification_path/tests.rs` (A796). The fixtures are openssl-signed P-256 test vectors. **Plants**: the
implied explicit policy removed fails two tests; the holder ignored fails two; the CA refusal off fails one; the TR
fallback removed fails one. Reversed, all pass.

**Gates.** fmt `--check` (`pdf-signature`): exit 0. clippy `--all-targets -D warnings`: exit 0. nextest of
`pdf-signature`: 490 of 490, 5 skipped. Fuzz `cargo check --bins`: exit 0. `cargo test -p conformance`: exit 0.
`state.sh --round 1993 docs`: exit 0, finished after the hand-off. Tier 2 (`corpus`, `raster_golden`): exit 101 at HEAD and after, with the same 62 incomplete documents and the same 16 moved pages, so no difference.

**Left of the row**: an attribute certificate's own revocation, signed assertions and `aaCertAlgorithmConstraints`
(ADR 2797); value constraints, which need a matching rule per type; trusted-list anchors; and ADR 2725's limits.
