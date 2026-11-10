# 1982 — A signature policy's time rules bound which revocation material answers

Session 1982, a single ledger round (§12.8.3.4.4), the second ledger lane. ADR 2775; 2776 not used; no question.
**The slice, and why.** Of the brief's rows, 7.4.9 is held by the owner's A348 (CIEJab) and 7.6.6 by A66's
trigger (the public-key handler); only §12.8.3.4.4 had effort-only work, and ADR 2725's table named it. Its
two time rows are one construction: TS 119 172-2 clause 4.23.3's freshness and TR 102 272 clause 6.8's caution
period both bound *when the material that answers a revocation check was issued*, and 6.8's delay is the finding
TS 119 172-3 clause 4.25 already makes. Retention, attribute certificates, trusted lists and a policy-supplied
path are different constructions and did not fit beside it.

**The premise held.** `revocation_checks` named `FRESHNESS`, and TR 102 272's members `[3]` and `[4]` were pushed
as names; nothing compared an issuance. The texts are held (`doc/ETSI_*`), and the duration arithmetic is XML
Schema Part 2 Second Edition (TS 119 172-3's reference [8], fetched from the W3C).

**Built.** `policy_rules::duration`: section 3.2.6.1's lexical form and Appendix E's sum, seconds rounded up as a
signed value (exact against whole-second instants). `revocation::Issuance` (`fresh_after`, `issued_from`) and
`status_issued`: a good answer only from material whose `thisUpdate` meets the bounds, EN 319 102-1 clause
5.2.5.4's stated maximum replacing the list's own window; a revocation answers whenever it was issued;
`Undetermined::IssuedBeforeRequired`. `revocation_checks::Freshness`, `Timing` and `Requirement::freshness`,
read in both TS syntaxes; best-signature time is EN 319 102-1 clause 5.5.4's. TR 102 272 clause 6.8: the delay
reuses `ts_119_172_3::delay_finding`; the caution period bounds both conditions' checks and is its own
`Constraint::CautionPeriod`, not evaluated before it ends (Annex B.5.3). `Verdict::word` keeps `fmt` in bounds.

**Tests.** `duration/tests.rs` (Appendix E's example and its pinning, the lexical form, the rounding);
`revocation_checks/tests.rs` (maximum freshness, the window replaced, time after signature at its boundary,
the caution period, no revocation withdrawn); `policy_rules/tests.rs` (TR delay; TR caution over the authority's
checks and as its own finding); `ts_119_172_2/tests.rs`'s freshness now evaluated (`P1D`, `P2M`).
**Plants**, each applied and reversed: every issuance admitted (five tests fail); TR's members named again and a
month read as thirty days (four fail); reversed, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` of `pdf-signature`: exit 0. nextest of
`pdf-signature`: 456 of 456, 5 skipped. `cargo check -p viewer-core --tests` (the one dependant that reads the
findings): exit 0. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh --round 1982 docs`: exit 0.
Tier 2: `pdf-model corpus` exit 101 as at HEAD (974 documents, 62 incomplete against a ceiling of 60); no other
line was run inside the wall, and no corpus signature states a machine-readable policy (ADR 1709).

**Left of the row**: retention of expired revocation notices, attribute certificates and their trust condition,
trusted-list anchors, a policy-supplied certification path, and the limits ADR 2725 lists.
