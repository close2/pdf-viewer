# 1885 — A signature policy in TS 119 172-2's XML syntax is read by a third parser and enforced over its signature

Slot 5 of batch 129, 2026-10-29, Opus 5.5, two sessions; ADR 2604 used, ADR 2605 and Q761 unused. Ledger: §12.8.3.4.4
stays `partial`, its residue narrowed to human-readable policies, the copy fetched at the URL qualifier, and an XML
policy bound only through its canonicalised `Digest` (A713).

**Premise.** The hypothesis held in its shape: a third parser behind the same findings, every rule read into the
TS 119 172-3 types and judged by one `judge_rules`. "The XML reader this tree already has" held for the tokenizer
only: `pdf-model`'s XFA tree is crate-private and `pdf-signature` may not depend on `pdf-model` (ADR 1020), so
`xmlparser` is reused and the module folds its tokens into a tree of its own, with namespaces resolved. Two premises
were wider than the brief: identifiers are URIs (an algorithm may be an XML Signature URI, so fifteen are placed by a
table checked against XML Signature 1.1, RFC 9231 and RFCs 3279, 4055, 5754, 5758), and clause 4.3's `Digest` is over
canonicalised XML, so binding is by the octets' digest alone and a signed `DigestValue` is said by name.

**Built.** `crates/pdf-signature/src/policy_rules/ts_119_172_2.rs` and its tests: `Reading::parse` (byte budget, depth
at `der::MAX_DEPTH`, every element, declaration and list token charged to `MAX_ENTRIES`, DTD refused) and
`Reading::document`; `findings`. `policy_rules.rs`: `Syntax::Ts1191722`, `Syntax::of` sniffs markup, three refusals
(`Xml`, `NestedTooDeep`, `ElementMalformed`), `NotEnforced::CanonicalDigestNotComputed`, `enforce_xml`; both ASN.1
parsers refuse XML as `OtherSyntax`. `ts_119_172_3.rs`: `Identifier` and `ExtensionName`, `judge_rules` split out of
`findings`, an empty `usages` read as every usage, `UnplacedAlgorithm` and `AlgorithmExpiration` named not evaluated.
`notes.rs` says the canonical-digest case; `lib.rs`, `Cargo.toml`, the row, `doc/todo/65` and `doc/todo/01` follow.
Fuzz target `policy_document`: one input to all three parsers, the accepting one `Syntax::of`'s, findings equal twice;
seeded by `fuzz/seed_policy_document.py` (28 documents); ADR 2604 states `xmlparser` as a stack decision.

**Plant.** An XML policy whose signing period ended 2026-06-01 against a timestamp of 2026-07-01: one `Broken` finding,
the signing period; against the fixture's real signature, `PolicyConstraintBroken` naming the period, the word valid
withheld; the same octets under both ASN.1 parsers: `OtherSyntax`, naming each. With `findings`' signing-period line
turned into a `drop` by a patch, both tests failed; reversed, both pass. Memory: the hostile probe peaked at 6 572 KiB
in 2.50 s by `getrusage`, against 5 652 KiB for TS 119 172-3's probe and 5 328 KiB for a unit test in the same binary.

**Unfinished.** XML canonicalisation for the `Digest` route; human-readable policies; enforcement over a fetched copy.

**Gates.** `cargo fmt --check` `-p pdf-signature`, `-p viewer-core` and `fuzz/`: 0; clippy `-D warnings --all-targets`:
`-p pdf-signature` 0, `-p viewer-core` and `fuzz/Cargo.toml` 101 in the worktree on a sibling's
`pdf-archive/src/examination.rs:248`, 0 in an export of HEAD and this round's files; nextest `-p pdf-signature` 0 (285
passed, 4 skipped), `-p viewer-core` 0 (434 passed, 2 skipped); `cargo doc -p pdf-signature` 0; `cargo deny check` 0;
`policy_document` built `-s none`, 601 s under the bound: 39 773 946 runs (66 179/s), no crash, peak 35 MB, 28 seeds
grown to 3 260 files, coverage 940 to 3 114 edges; `cargo test -p conformance --no-fail-fast` 101, 469 passed and 1
failed (`records`, this paragraph, since filled); `--test fuzz_workspace` 0 (4 passed); `--test records --test
adr_status --test ledger_notes` 0 (8 passed); `--bin cited` and `--bin quotations` 0, twice.
