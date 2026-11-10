# 1988 — The presence and kind of signer attributes are judged in all three syntaxes

Session 1988, a single ledger round (§12.8.3.4.4), the second ledger lane. ADR 2787; no question.

**The slice, and why.** ADR 2786 ordered presence and kind first, in all three syntaxes. It is a whole construction:
one reader of what a signature carries, judged by TR 102 272 clause 6.9 and by clause 4.26 of both TS 119 172 parts.
The RFC 5755 reader was not begun. With nothing judged over it yet it would be a placeholder, and the certified half
(the authority's path, the holder, validity and revocation) is one round's work of its own.

**Premise.** It held. TR 102 272's `attributeTrustCondition` was pushed as a name and never read. Both TS parsers
counted `AttributeSetConstraints` and judged none, and no code read a signer-attributes value except §12.8.3.4.3 (j)'s
emptiness check. The clauses decide every case built, so effort was the only obstacle.

**Built.** `policy_rules::signer_attributes`: `carried` reads the signed `signer-attributes-v2` (EN 319 122-1 clause
5.2.6.1) and `signer-attributes` (RFC 5126 section 5.11.3) under `EXPLICIT TAGS` and `MAX_SIGNER_ATTRIBUTES`. It
yields each claimed attribute's type and a count of attribute certificates and signed assertions; a value it cannot
read is named and not evaluated. `judge_condition` decides clause 6.9's `attributeMandated` against
`attributeTypeConstraints`, where a type only `attributeValueConstraints` names is undecided. `judge_set` decides
clause 4.26's level over the attributes a set applies to, and each mandated identifier. A certificate or an assertion
is not evaluated wherever only its contents would decide. Value constraints are named. The ASN.1 parsers read
`AttributeTrustCondition` and `AttributeSetConstraints`, and the XML parser reads `AttributesSetConstraintsType`, its
RFC 3061 URNs placed as object identifiers. Found on the way: §12.8.3.4.3 (j)'s emptiness check and its fixture read
the v2 lists as implicitly tagged. Both now follow the module.

**Tests.** `signer_attributes/tests.rs` (eight tests: both syntaxes read, malformed shapes, an unsigned copy, the
bound at its edge, and the level and identifier rules), `policy_rules/tests.rs` (two tests, TR 102 272 end to end), and
one end-to-end test each in `ts_119_172_3/tests.rs` and `ts_119_172_2/tests.rs`, plus one refusal test. **Plants**:
a refused kind counted as met fails one test; unsigned copies read fails one; type constraints ignored fails two;
the implicit emptiness check restored fails §12.8.3.4.3's test. Reversed, all pass.

**Gates.** fmt `--check` of `pdf-signature`: exit 0. clippy `--all-targets -D warnings`: exit 0, with
`CARGO_INCREMENTAL=0` after a clippy ICE under incremental. nextest of `pdf-signature`: 481 of 481, 5 skipped. Fuzz
`cargo check --bins`: exit 0. `cargo test -p conformance`: exit 0. rustdoc `-D warnings` of `pdf-signature`:
exit 0. `state.sh docs`: exit 101 from a rustdoc ICE in `conformance` under incremental; its `CARGO_INCREMENTAL=0` re-run, after the hand-off: exit 0. Tier 2
(`corpus`, `raster_golden`), run at the merge: as at HEAD.

**Left of the row**: certified attributes and signed assertions under their trust condition (an RFC 5755 reader, the
authority's path, the holder, validity, revocation, and the types they carry); value constraints, which need a
matching rule per type; trusted-list anchors; and ADR 2725's limits.
