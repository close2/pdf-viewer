# 2044 — A 1-Pass ECMQV recipient opens, and a PSS suite is read by its mask

Session 2044, a single ledger round, the signatures lane (§7.6.5.3 and §12.8.3.4.5's residue,
`doc/todo/77` item 3). ADRs 2904 and 2905; no question.

**The slice, and why.** The brief's three items in its order, each whole: they are the effort-only
remainders of the two rows, and all three fitted the budget; nothing new was opened.

**The premise.** Held: RFC 5753's five `mqvSinglePass-*` were refused by identifier; `Family::RsaPss`
carried no MGF1 hash, so Table 4's PSS rows were chosen by the digest alone; no time-stamp imprint was
assessed. brainpoolP512r1 re-checked on 2026-11-16: no `bp512` on crates.io or in RustCrypto's
repository, `bp512-nestler` is PolyForm Noncommercial; not built. No `A*` file cites any of the three.

**Built.** `pdf_pki::ec`: SEC 1 section 3.4's MQV primitive run by the static-only party (SEC 1
section 5.2, SP 800-56A section 5.7.2.3.2), `id-ecMQV` keys read, RFC 5480 section 2.1.2's
restriction kept (`EcError::Restricted`). `enveloped::agreement`: RFC 5753 section 3.2's recipient,
`MQVuserKeyingMaterial` from `ukm`, `entityUInfo` = `addedukm` (section 7.2). `pdf_signature`:
`Family::RsaPss { mask }`; Table 4's PSS row needs the MGF1 hash in the row as well as the digest;
clause 9.2's `Assessment::imprints` over every token the report takes as a time; `notes` names it.

**Tests.** NIST CAVP's KAS ECC responder vectors (P-256, P-384, P-521), restriction tests, and 13
envelopes `r2044/gen/gen_mqv.py` wrote on keys it generated (its constants calibrated by
`(n-1)G = -G`); two `ts_119_312` tests; one report over a document whose second document timestamp
covers the first. **Plants**: the half-bit length floored and `entityUInfo` read from `ukm` failed 4;
Table 4's mask condition disabled failed the PSS test; `assessment.imprints` dropped from
`Report::of` failed the report test. Reversed, all pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-pki`, `pdf-syntax`, `pdf-signature`,
`viewer-core`): 0. nextest of the four: 1603 of 1603, 8 skipped. Fuzz `cargo check --bins`: 0.
Rustdoc `-D warnings --document-private-items` on the three touched: 0. Conformance after
`git add -N`: 474 passed. Tier 2: `corpus` 101, 974
documents, 67 incomplete (ceiling 65), joined issue15053 and mixedfonts as at HEAD; `raster_golden`
101, the same 23 names moved as at HEAD; `selection_census` 0 (974); `accessibility_census` 0 (1021);
`awkward_classes` 0 (152 swept of 2677, none killed); `state.sh --round 2044 docs` 0.

**Habit, and what is left.** A curve constant typed into a generator is calibrated against an
independent scalar multiplication (`(n-1)G = -G`) before any fixture is written; it caught a
mistyped P-521 order. §7.6.5.3 stays `partial` for brainpoolP512r1 alone; §12.8.3.4.5 stays
`implemented`, left clause 9.3 and 9.5's key lifetimes and national rules (none held).
