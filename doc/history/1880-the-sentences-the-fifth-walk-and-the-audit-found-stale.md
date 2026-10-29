# 1880 — The sentences the fifth walk and the audit found stale, rewritten to what the tree does

Slot 10 of batch 128, 2026-10-29, Opus 5.5, one session, a docs round; ADRs 2594 and 2595 and Q756
unused. Ledger: no row moved — no behaviour changed, and §12.8.3.4.4's row is round 1873's.

**Premise.** The eleven sentences held, each read at its line and checked against the code or the
owner's `A` file before it was rewritten. The brief's lines were narrower than the tree's (trap
25): the same false claims stood beside them, and a sweep of `pdf-signature` for "answers none",
"not taken", "no host" and "today" found eight more, rewritten with them — `signature.rs`'s
`Authenticity`, `SigningCertificateBinding` and `trust`'s last paragraph, `x509.rs`'s opening line
and its `extended_key_usage` field, `dsa.rs`'s step 5, `revocation.rs`'s `Revocation` and
`Material::none`, and `trust.rs`'s step (a)(3) comment; `integer_entry_census.rs` called Q31 open,
and `doc/todo/58` called A07's scheme and A08's write question open beside A09's rename.

**What each now says.** Question 3 is answered under anchors a host supplies (`--trust-anchors`,
`viewer_host::trust_anchors`, `Supply::read`) with revocation from the document's own CRLs and OCSP
responses, applied by `trust::validate` to every certificate on the path; `x509` reads the fields a
path turns on and `trust` alone weighs them; `policy.rs` says both ASN.1 syntaxes are held and
enforced by `policy_rules` (ADR 2560); `notes.rs`'s closing sentence for a reading with no anchor
drops "It can now say" and "which nothing here does" for "which nobody has done for this reading",
and its test's pinned substring follows it; `doc/HANDOVER.md` says RFC 0009 §12's items 1 and 3 are
built and item 2's printing is not; `integer_entry.rs` cites A31's option 2 as answered and unbuilt
(ADR 2564); `doc/todo/58` §6 is four answered, two of them unbuilt, three standing.
`doc/third-party-data.md` carries the Adobe Supplement's section (A288's owed row): the file, its
source snapshot, 1 373 256 bytes and its SHA-256 re-measured here, its copyright page's terms.

**Plant.** `notes.rs`'s string set back to "which nothing here does" with the test left as
rewritten: `a_document_carrying_a_security_store_says_what_is_in_it_and_claims_nothing_from_it`
failed at `notes.rs:2819`, exit 101; restored by the reverse replacement and green.

**Gates.** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p pdf-signature -p viewer-core -p
pdf-model` exit 0; `rustfmt --check --edition 2024` on the ten Rust files exit 0; `cargo test -p
viewer-core --lib notes::` 19 passed; `tools/comment-history.py` 3 history lines, none of them
mine; `RUSTFLAGS="-D warnings" cargo clippy --no-deps` on `pdf-signature` and `viewer-core`
`--all-targets` and `pdf-model` `--lib --examples` exit 0 (`pdf-model`'s `--all-targets` stops at two
siblings' new test files, `pdf-font`'s lint at a sibling's `glyphs.rs`, none mine); `cargo test -p
conformance --no-fail-fast` exit 0, 470 passed, the three history gates among them, and once more
with this record final: exit 101, 469 passed, the one red `records` on round 1876's record in progress; `--bin pointers`, `--bin cited` and `--bin quotations` exit 0, none naming a file of mine.
