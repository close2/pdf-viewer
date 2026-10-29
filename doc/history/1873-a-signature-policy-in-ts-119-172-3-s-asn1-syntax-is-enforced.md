# 1873 — A signature policy in TS 119 172-3's ASN.1 syntax is read by a second parser and enforced over its signature

Slot 3 of batch 128, 2026-10-29, Opus 5.5, two sessions (the first handed over past 150k context); ADR 2580 used, ADR
2581 and Q749 unused. Ledger: §12.8.3.4.4 stays `partial`, its residue narrowed to TS 119 172-2's XML, human-readable
policies and the copy fetched at the URL qualifier (A713).

**Premise.** The hypothesis held: a second parser behind ADR 2560's `Finding` vocabulary, told apart by its first
member, with `enforce` grown by a dispatch. One premise did not: "the same DER budget". TS 119 172-3 clause 4.1.3.2
admits BER, so the first syntax's DER pre-pass would refuse documents the standard writes; the budgets are the same
byte and entry counts, with nesting held by `der::Reader` as each value is opened.

**Built.** `pdf_signature::policy_rules::ts_119_172_3`: Annex A's `SignaturePolicy`, every `BasicRule` alternative,
commitment rules with their matching indicator, each rule's scope, the `Digest` re-computed over `PolicyComponents`
(`DigestDisagrees` when it differs). `findings` enforces the signing period, media types, cardinality, relative
position, signer certificate, timestamp delay, prohibited signer attributes and the cryptographic suites of the
`Signature` and `SigningCert` usages; the rest is named not evaluated. `policy_rules.rs`: `Syntax` and `Syntax::of`,
`OtherSyntax(Syntax)`, `Enforcement::Enforced { syntax, findings }`, `enforce_components`, a shared signing-period and
commitment-type judgement, `SignerFacts::certificate_signature_algorithm`. `viewer-core`'s `notes.rs` names the syntax
in its sentence. ADR 2580 records the binding choice (EN 319 122-1 clause 5.2.9.1 leaves the hash input to the policy's
text, TS 119 172-3 names none), the readings per clause and the `Usage/#SigningCert` spelling accepted both ways.

**Plant.** A TS 119 172-3 document whose signing period ended 2026-06-01, against a signature timestamp of 2026-07-01:
`Reserved(PolicyConstraintBroken)` naming the period, the word valid withheld; the same octets under TR 102 272's type:
`OtherSyntax(Tr102272)`. Kept as `a_signing_period_that_ended_before_the_signing_time_is_broken`. Memory: the ignored
hostile probe peaked at 12 380 KiB in 0.11 s by `getrusage`, beside 12 180 KiB (the first syntax's probe) and 12 204 KiB
(a floor test) in the same binary.

**Unfinished.** TS 119 172-2's XML syntax; human-readable policies; enforcement over a copy fetched at the URL.

**Gates.** In export `r1873b` (HEAD plus this round's five source files): `RUSTFLAGS="-D warnings" cargo clippy -p
pdf-signature -p viewer-core --all-targets` exit 0 (58 s); `cargo test -p pdf-signature -p viewer-core` exit 0, 698
passed, 0 failed, 5 ignored (101 s); `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p pdf-signature` exit 0 (5 s),
again in the worktree after `lib.rs`'s line, exit 0 (46 s). Session 1 in the worktree: `cargo nextest run -p
pdf-signature --lib -E 'test(policy_rules)' --run-ignored all` exit 0, 31 of 31. `rustfmt --edition 2024 --check` on
the five files exit 0. `--bin cited` exit 0 (2 s); `--bin quotations` exit 0 (191 s), no divergence in this round's
files; `cargo test -p conformance --no-fail-fast` exit 0, 470 passed, 0 failed (261 s), then `--test records --test
adr_status --test ledger_notes` after this record and ADR 2580, exit 0, 8 passed. The fuzz target is not owed: `der.rs` is unchanged.
