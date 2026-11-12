# 2012 — A trusted list is authenticated before it is believed

Session 2012, a single ledger round, the signatures lane (`doc/questions/A800` item 3,
`doc/todo/77` item 1; §12.8.3.4.5, §12.8.3.4.4). ADRs 2842 and 2843; no question.

**The slice, and why.** `doc/todo/77`'s first item whole: ETSI TS 119 615 clause 4.1 (the list of
trusted lists against the Official Journal's certificates, through its pivots) and clause 4.2 (a
Member State's list against the LOTL's pointer), over an XML Signature 1.1 verifier. Every level
rests on it (ADR 2835); the QC statements did not fit beside the tier-2 walks.

**The premise.** Held: `Authentication` had one value, `VouchedForByTheHost`; no `ds:Signature`
reader; `canonical_xml` chose its apex by name and depth only and bounded at a policy's 256 KiB.
Nothing owner-blocked, so no Q843; no `A*` after A800 cites the item. Found: a host given
`--trusted-lists` without `--trust-anchors` asked every verdict at the epoch; fixed (ADR 2842 §6).

**Built.** `canonical_xml::canonicalize_subset` (an apex by ordinal, an excluded element, a whole
document, the caller's budget). `xml_signature`: core validation in TS 119 612 Annex B's shape (RSA
PKCS#1 v1.5, RSASSA-PSS, ECDSA `r || s`; wrapping shapes refused; `SigningCertificateV2` compared).
`trusted_list`: clause 5.3.7's URIs, 5.3.13's pointers; `authentication` (PRO-4.1.4-01 to -15,
PRO-4.2.4-01 to -12) over the host's files as ADR 2842 names them; `Lists::read(at)` gives a
`Reading`; `Authentication::Authenticated`. `notes` words the outcome, each refusal and warning, and
drops "believed" for an authenticated listing. `fuzz/` `trusted_list` runs both clauses.

**Tests.** Ten for the verifier, eight for the clauses (pass, a changed list, another signer, one
outside the Journal's set, -03/-10/-13, a pivot and its absence, vouched, PEM), one verdict, one
`notes`; lists signed in the test over canonical forms written by hand. **Plant**: the enveloped
exclusion and PRO-4.1.4-12 disabled: 10 of 19 fail; reversed, they pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-signature`, `viewer-core`, `viewer-host`):
0. nextest of the three: 1367 of 1367, 8 skipped; doctests 0. `cd fuzz && cargo check --bins`, fuzz
clippy: 0. Rustdoc `-D warnings --document-private-items`: 0. Conformance after `git add -N`: 472
passed, once a doc path through a `pub use` alias took its declared name. Evidence: the real LOTL,
its five pivots and Austria's list verify; Germany's (`id-RSASSA-PSS` key) is refused by name
(`doc/todo/77` item 0). Tier 2: `corpus` 101, 974 documents, 61 incomplete, round 2008's pair joined
(`issue15053.pdf`, `mixedfonts.pdf`); `raster_golden` 101, held 951, moved 23, round 2008's 23
names, no drawing path reached; `selection_census` 0 (974 documents); `accessibility_census` 0,
`awkward_classes` 0 and `state.sh --round 2012 docs` 0, each run after round 2011's walks.

**Left of the rows.** §12.8.3.4.5 stays `implemented`; §12.8.3.4.4 `partial`, `TAsInTrustedList` its
list residue. `doc/todo/77` holds the rest, the QC statements first.
