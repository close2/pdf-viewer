# 487 — The padding inside the family called done

**Finding.** `id-RSASSA-PSS` — the commonest thing this program declined, twice ECDSA's share,
sitting *inside* Table 260's "RSA Algorithm Support" row this tree called verified — is now
verified: `crates/pdf-model/src/pss.rs` is RFC 8017 sections 8.1.2 and 9.1.2 with Appendix
B.2.1's MGF1 over the modular exponentiation `bigint` already performs, parameterised by the
`RSASSA-PSS-params` the signature's own algorithm identifier carries, kept deliberately separate
from PKCS #1 v1.5 with only RFC 8017's own shared primitive (`pkcs1::rsavp1`) between them. The
census's six real PSS witnesses moved from `AlgorithmNotVerifiable 1.2.840.113549.1.1.10` to
`Verified (2048-bit RSA (RSASSA-PSS))`, six for six, with every other census line byte-identical.
Parameters this program cannot act on — a hash outside RFC 8017's set, a mask generation function
other than MGF1, a trailer field other than 1, absent parameters — are refused by the file's own
numbers through `Authenticity`, never defaulted around. The verifier is pinned two ways: openssl
vectors on a key made for the tests, and section 9.1.1's encode operation implemented from the
RFC's own steps, which reaches the `emLen = k - 1` edge openssl cannot make a key for.

**Date.** 2026-08-14.
**ADR.** [0322](../adr/0322-the-padding-inside-the-family-called-done.md).
**Touched.** `crates/pdf-model/src/pss.rs` (new), `crates/pdf-model/src/pkcs1.rs` (`rsavp1`
extracted, module doc), `crates/pdf-model/src/cms.rs` (`SignatureAlgorithm::RsaPss`,
`signature_algorithm_parameters`, the `detached_pss` fixture), `crates/pdf-model/src/signature.rs`
(`Family::RsaPss`, `Authenticity::PssParametersNotVerifiable`, the PSS arm of `authenticity`, an
end-to-end test), `crates/pdf-model/src/lib.rs`, `crates/pdf-model/examples/signature_algorithm_census.rs`,
`crates/viewer-core/src/notes.rs` (the sentences naming what is verified),
`fuzz/fuzz_targets/x509.rs` (PSS added to the never-verifies property), five §12.8 rows in
`doc/conformance/ledger.toml`, `doc/todo/51-signatures-and-public-keys.md` (item closed, retitled),
`doc/HANDOVER.md`, `doc/crate-map.md`, `doc/verify.md`, `doc/adr/0322-*` (new), this file.

The gates: fmt, clippy (silent), nextest workspace, doctests and the conformance checker, run
after the final edit. The corpus and oracle raster gates were not run: no gate in this tree looks
at a signature and nothing this round touched can reach a raster (ADR 0215's reasoning, unchanged
through 0229, 0314 and now 0322). The `cms` and `x509` fuzz targets were re-run seeded and clean —
`cms` because `read_signer_info` now captures the algorithm parameters, `x509` because the target
gained the PSS never-verifies assertion.
