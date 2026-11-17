# 2053 — A signer's SAML assertion is read, its signature verified, and judged under a policy's set

Session 2053, a single signatures round in the gwg worktree. Row §12.8.3.4.4; ADRs 2922 and 2923.

**The slice.** What round 2039 left as work (ADR 2895): ETSI EN 319 122-1 clause 5.2.6.3's signed SAML
assertion, and clause 5.2.6.2's claimed one beside it, read and shown, and judged where a TS 119 172 set
accepts signed assertions.

**The premise.** It held: `signer_attributes` counted signed assertions and never opened one, and
`Check::Assertions` said so. Nothing owner-blocked, so no Q881. EN 319 102-1 clause 5.2.8.4.2.7 decides what a
verifier owes: an unconstrained assertion made available, a constrained one validated. SAML core section 2.3.3
requires the assertion's own XML signature verified before its contents are relied on, so it was built; it
needed no new code beyond a second entry point to `xml_signature` (ADR 2843's canonicaliser), and no dependency.

**Built.** `saml.rs` reads SAML core section 2.3.3's assertion under a 64 KiB, 4 096-entry and 64-attribute
budget; `xml_signature::verify_saml` applies section 5.4's profile (one reference to `#ID`, enveloped, only
exclusive canonicalisation, the signer the `KeyInfo` certificate whose key verifies, no key said as unverified).
`signer_attributes::asserted` judges a signed assertion by clause 5.2.8.4.2.7: read, signed, verified, its
signer's path to the set's trust trees, section 2.5.1's conditions at the instant, the set's revocation
requirement. A SAML attribute names a policy's identifier only under the URI name format (ADR 2923). The
applicability report's item f) carries each assertion, and `viewer-core` says it, withholding the contents of
one whose signature is invalid.

**Tests.** Eight in `saml/tests.rs` from SAML core's own sections over fixtures signed here in canonical form;
four judgements in `signer_attributes/tests.rs` (valid, past `NotOnOrAfter`, tampered and unsigned, trust trees
absent or elsewhere, a refused kind by its URI names); one sentence test in `viewer-core`. **Plant**:
`SignatureCheck::is_invalid` made always false failed five tests (three in `saml`, one judgement, the sentence);
reversed, all pass.

**Gates.** Over `pdf-signature` and `viewer-core`: fmt exit 0; clippy `-D warnings --all-targets` exit 0;
nextest 1187 of 1187 passed, 7 skipped; rustdoc `-D warnings --document-private-items` exit 0 (after one private
link was unlinked). Fuzz, where `cms` read the old count and the new `saml` target was added with its
`doc/verify.md` line and `fuzz/seeds.sh` arm: fmt, `cargo check --bins` and clippy `--bins` exit 0. Conformance
after `git add -N`: exit 101 on a dropped quote in the ledger row, the new target's two registrations and this
paragraph, then exit 0. No tier 2: nothing draws or walks a corpus here.

**Left.** The row stays `partial` on its other items. A refused assertion's scope reads an unverified one's
attributes; `doc/stack.md`'s XML row does not yet name SAML among `pdf-signature`'s readers.
