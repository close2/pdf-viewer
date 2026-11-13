# 2017 — A PSS key keeps its restriction, and a certificate's statements are named

Session 2017, a single ledger round, the signatures lane (`doc/questions/A800` item 3, `doc/todo/77`
items 0 and 1; §12.8.3.4.5). ADRs 2852 and 2853; no question.

**The slice, and why.** `doc/todo/77`'s items 0 and 1 together: the RFC 4055 key that kept Germany's
list unauthenticated, and EN 319 412-5's statements clause 4.4 starts from. Item 2 (TS 119 615
clause 4.4, a thousand lines of processing over TS 119 612 clause 5.5.9.2) is a round of its own and
would have changed `pdf-signature`, which `pdf-model` builds, under queued walks.

**The premise.** Held: `read_public_key` carried `id-RSASSA-PSS` as `Unverifiable`; no reader of
`qcStatements`; `Listing` held no certificate claim. Nothing owner-blocked, so no Q848; no `A*`
after A800 cites either item.

**Built.** `pdf_pki::x509`: `PublicKey::RsaPss { key, parameters }` (RFC 4055 section 1.2),
`PublicKey::pss_parameters`, `ID_RSASSA_PSS`, `Extensions::qc_statements` beside the path.
`pss::admitted_by_key` is section 3.3, asked by the CMS, certificate, CRL, OCSP and XML Signature
verifiers; a PKCS #1 v1.5 signature under such a key is refused. `qc_statement` reads clause 4.2's
four by their syntax, names the rest, bounded; `Listing::statements` carries them
(`Standing::Listed` boxed); `notes` names them as the issuer's claims, never a level. `fuzz/`
`qc_statement`, and `x509` reaches the new key.

**Tests.** pdf-pki: the key read, absent and present parameters. pss: section 3.3's three cases.
xml_signature: two lists under generated PSS keys (signed in the test by RFC 8017 section 9.1.1).
qc_statement: five, over octets written from the clauses and an `openssl`-made certificate. verdict
and notes: one each. **Plant**: the salt comparison and `QcSSCD` disabled: 3 fail; the key reader
disabled: 1 fails; reversed, they pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-pki`, `pdf-signature`, `viewer-core`): 0.
nextest of the three: 1114 of 1114, 7 skipped; doctests 0. `cd fuzz && cargo check --bins`, fuzz fmt
and clippy: 0. Rustdoc `-D warnings --document-private-items`: 0. Conformance after `git add -N`:
472 passed once two Table citations named EN 319 412-5. Evidence: Germany's list verifies and is
authenticated under LOTL 398 beside Austria's. Tier 2: `corpus` 101, 974 documents, 61 incomplete,
round 2015's names; `raster_golden` 101, held 951, moved 23, round 2012's names, no drawing path
reached; `selection_census` 0 (974); `accessibility_census` 0 (1020); `awkward_classes` 0 (152 swept
of 2677, none killed); `state.sh --round 2017 docs` 0.

**Habit, and what is left.** A comparison of several members needs a fixture per member where only
it differs: the salt plant passed the end-to-end refusal, which differed in the hash too.
§12.8.3.4.5 stays `implemented`, §12.8.3.4.4 untouched; `doc/todo/77`: TS 119 615 clause 4.4 first.
