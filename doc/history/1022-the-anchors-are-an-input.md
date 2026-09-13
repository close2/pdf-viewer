# 1022 — The anchors are an input

Contract: §12.8's trust-store debt — check the scope, decide what a trust store means here, build
the first slice.

**The scope was undercounted.** §12.8 has 36 rows: 9 `implemented`, 22 `partial`, 5 `reported`. The
measurement said twelve of the 22 were this debt; **it is fourteen**, and the five `reported` are
the same debt, so the span is **nineteen of the 27 non-`implemented` rows**. Solely trust, nine
`partial`: §12.8, §12.8.3.2, §12.8.3.4, §12.8.3.4.5, §12.8.4, §12.8.4.2, §12.8.4.4, §12.8.5,
§12.8.5.2. Trust plus a second debt, five: §12.8.1 (Table 256's `/DigestMethod`), §12.8.3,
§12.8.3.1, §12.8.3.3, §12.8.3.3.1 (four of ISO/TS 32002's eight curves). Not this debt, eight:
§12.8.2 to §12.8.2.4 are §12.8.2.2.2's revision comparison, §12.8.3.4.2 is `der.rs`'s
indefinite-length tolerance, §12.8.3.4.3 documents this tree lacks. **And the trust store closes
none of the nineteen** — each also needs revocation, a curve or the revision walk.

**The decision is ADR 1039** and was made before any code: a trust store here is an *input*, not a
list. RFC 5280 section 6.1.1 makes the anchors input (d) and calls their selection "a matter of
policy"; `CLAUDE.md` principle 3 puts a policy where a host can supply it. A compiled-in root list
and the platform's store are both priced and both refused as *defaults* — each is a TLS
server-authentication programme, which a document-signing decision is not; the platform's also
needs a filesystem the renderer has not got and makes a verdict a function of the machine.

**Built**: `crates/pdf-signature/src/trust.rs`, RFC 5280 section 6.1 with path building over
host-supplied anchors and a caller's instant — (a)(1) the issuer's signature over each
`tbsCertificate`, (a)(2) validity, (a)(4) chaining, (k) `basicConstraints`, (l)/(m)
`pathLenConstraint`, (n) `keyCertSign`, (o) unrecognised critical extensions as a refusal; bounds
8/64/256 and no certificate twice. `x509.rs` gained what those steps read. `Signature::trust` is
where §12.8 asks. **Not built, and said in the types**: revocation (`Revocation::NotChecked`, one
variant, no answer says *valid*); the policy tree, omitted under section 6.1's own permission and
safe because `requireExplicitPolicy`, name constraints and inhibit anyPolicy each arrive critical;
byte name-matching, which can only fail a path that should chain. **No host supplies an anchor**, so
nothing a person sees changed and no row moved — ADR 1039 says so rather than leaving it found.

**Calibrated** (trap 13): fourteen unit tests over a hierarchy `openssl` issued for this, each
negative one change from the positive, and planted both ways — disabling (a)(1) fails
`one_turned_bit_…`, forcing verification false fails the corpus gate's `Anchored` assertion.
`every_corpus_signature_is_asked_the_third_question_both_ways` runs section 6.1 on real chains:
`xfa_filled_imm1344e.pdf` validates to its own root, two are `NotCurrent` at the fixed instant.

**Gates.** Tier 1: fmt 0 · nextest 4548 passed 0 · doctests 0 · `-p conformance` 0 · both `fuzz/`
lines 0, and the x509 target grew `trust::validate` (60 000 runs, INITED, cov 351 ft 635, no
crash) · workspace clippy 101, on `pdf-model/src/document_part.rs` alone — a sibling's new file in
this shared worktree; `clippy -p pdf-signature --all-targets` is 0.
Tier 2 (rule 2): `pdf-model` corpus/dates/xmp/jpeg2000 0 · `pdf-syntax` on_disk 0 · `pdf-transform`
gate 0 · `raster_golden` fails on `encrypted-attachment.pdf` p1 "was drawn, is locked" — a
sibling's `crypt.rs` change; this round touched no encryption, no page tree, no mark on a page.
