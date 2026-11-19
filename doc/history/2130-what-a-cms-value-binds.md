# Round 2130 — what a CMS value binds: audit 2128's group A

A fix round in the `gwg` worktree for review 2128's group A: B01, B02, T01, D01, D02 and D04.
Security first: these two let a forged or unrelated value clear §12.8.1's first two questions.

## What changed

- B01: `Signature::integrity` now compares the second link of the encapsulating formats. That
  link is the signer's `message-digest` against the encapsulated content (`content_binding`, RFC
  5652 sections 5.6 and 11.2), in the `adbe.pkcs7.sha1` branch and in the `TSTInfo` branch. A
  mismatch is `Changed`. The `TSTInfo` branch's test forges the openssl-issued token's imprint.
- B02: a value whose signature is not over the document's bytes and that records no digest of
  them reads the new `Integrity::NoDigestOfTheDocument`. `UnderTheSignersKey` is kept for a
  signature over the bytes. `Verdict::reached` checks that premise in question 2.
- T01: the `encapsulating` fixture's attribute is the content's digest. A wrong one is a second
  case that expects `Changed`, and the test is renamed `..._against_its_content_and_its_attribute`.
- D01, D02, D04: `Signed::EncapsulatedContent`, the verdict's comment and ledger row 12.8.3.3.1's
  note now state what is true. `pdf-transform` and `viewer-core` each gain one sentence for the
  variant. ADR 3221 records the decision.

## Ledger and manifest

Row 12.8.3.3.1 stays `implemented`: the two findings that contradicted it are fixed, and it lists
the new tests. Row 12.8.1 stays `implemented`, with B04 still open against it. The manifest loses
the B01 and B02 entries. 2128's `findings.json` marks the six findings fixed and leaves the audit's
prose untouched.

**Gates.** Over `-p pdf-signature -p pdf-transform -p viewer-core`: fmt exit 0. clippy
`-D warnings --all-targets` exit 0 on the second run; the first found two lint shapes in new code.
`pdf-sandbox --bins` exit 0. nextest `--lib --bins --tests`: 2583 run, 2577 passed, 6 failed, the
manifest's remaining entries for these crates; the difference is 0 and 0. rustdoc `-D warnings
--document-private-items` exit 0 on the second run, after a private link. `cd fuzz && cargo check
--bins` exit 0. `cargo test -p conformance --no-fail-fast` passed except `batch`'s
`the_wrappers_last_line_names_the_seconds_gates_reads` (`sleep 1` took 3 s under load), which passed alone.

## Habit

A digest one link away from the signature is not recorded until every link is compared. The
timestamp path made the second comparison and the signature path did not. Two readers of the same
structure in one crate drift apart unless one calls the other.
