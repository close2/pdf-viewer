A time-stamping authority is held by its tokens, and a value by its type's rule

Round 2039, a single ledger round, the signatures lane (§12.8.3.4.4, what ADRs 2890 and 2891
left). TR 102 272 clause 6.10's `tsaCertAlgorithmConstraints` is judged on the algorithms the
authority made the signature's time-stamp token with, superseding ADR 2720's reading of it on the
authority's certificate, and TS 119 172's `Usage#Tstk` is judged the same way (ADR 2894). TR 102
272 clause 6.9's `attributeValueConstraints` is decided by X.501 clause 8.9.2.1 under X.520's
`caseIgnoreMatch` for the 26 `id-at` types that state it; signed assertions are read and recorded
(ADR 2895).

- The premise held: `tsaCert…` was judged on the certificate, `Usage#Tstk` was named as not
  evaluated, every value constraint was not evaluated, and no signed assertion was opened. Clause
  6.10's comments and TS 119 172-2 clause 4.29.1 agree on tokens. Nothing owner-blocked, so no Q866.
- pdf-signature: `policy_rules::token_made` and `TokenIssuance` (both algorithms listed, the
  minimum on the key that verifies the token); `ts_119_172_3::TSTK_USAGE`, `Usage::TimestampToken`;
  `signer_attributes::values` (the rule table, clause 7's preparation exact on ASCII, every value
  of an attribute against the listed ones); claimed and certified attributes keep their values.
- Left open by name: `id-at-role` (X.509 is not held), `dc` (its syntax is not its rule's), every
  type with another rule, and anything outside ASCII. TS 119 172's value constraints have no form
  (`Other` with no identifier, `AnyType`), and a signed assertion of a form other than EN 319 122-1
  clause 5.2.6.3's SAML one is undefined: both are limits. The SAML form is work: a SAML reader.

**Tests.** Two for the token lists (the minted token: `id-sha256`, `rsaEncryption`, RSA 2048, and
a copy whose signature no longer verifies), four in `values/tests.rs` from X.520's own examples
("Dundee"/"DUNDEE", clause 7.6.1's spaces, clause 6.4.3's title), and the clause 6.9 test rewritten
by value. **Plants**: the key never measured, and case not folded; six tests failed, and reversed,
all pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-signature`): 0. Nextest: 601 of 601, 5
skipped. Rustdoc `-D warnings --document-private-items`: 0. Fuzz `cargo check --bins`: 0.
Dependents check: 0. Fuzz: `cms` asks the value comparison of every claimed attribute; fmt, clippy
`--bins`: 0. Conformance after `git add -N`: 474 passed. Tier 2: `corpus` 101, 974 documents, 61
incomplete, HEAD's; `raster_golden` 101, held 951, moved 23, the same names as round 2037's; no
drawing path reached. `state.sh --round 2039 docs`: 0.

**Habit.** Two neighbouring members of one ASN.1 list are read by their own comments: where
clause 6.10 says "issuer of" for two lists and names the authority for the next two, the difference
decides what they constrain, and a later text that carries the list forward settles it.
