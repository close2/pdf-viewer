# 1053 — The material was in the file all along

Contract: §12.8.4 and §12.8.5 with their children, and the five `reported` revocation rows.

## What it found before building anything

§12.8.4.2 says the network was never the missing piece: a store exists so that "[a] PDF signature
may not be successfully verified unless its collateral validation components are preserved", and
§12.8.3.3.2 prints the *whole grammar* of `RevocationInfoArchival`. Two supplies, both inside the
file. The rows were wrong about the specification, not about the code.

## What was built

1. **`signature::security_store` reads Tables 261 and 262** into typed values: every stream
   decoded, every entry it would not take named by `StoreRefusal`. It was four counts.
2. **`revocation.rs`**: RFC 5280 section 5's `CertificateList` and RFC 6960 section 4.2's
   `BasicOCSPResponse`, on the crate's own DER reader, refusing indefinite lengths by name because
   both documents require DER. Plus §12.8.3.3.2's attribute.
3. **`revocation::status`** is RFC 5280 section 6.3.3, reduced on the RFC's own last paragraph and
   run inside `trust::Search::walk` for *every* certificate on the path; `Revocation` gains `Good`,
   `Revoked` and `Unknown`. ADR 1067 section 2 fixes the rule: absence of evidence is `Unknown`.
4. **`viewer-core::notes`** names the store's contents and each piece it could not use; the
   closing paragraph now splits the third question — revocation answerable from the file, trust
   answerable by nobody here.

## Two things the census caught that no fixture would

- **A refusal was aborting a search.** RFC 6960 criterion 2 is tried before criterion 3, and
  against a delegate's signature the CA's modulus is the wrong *shape*, so the arithmetic refuses
  by name — and a `?` there reported "this program does not verify 1.2.840.113549.1.1.11" about an
  algorithm it verifies. Two real documents said so. The delegate fixture was regenerated with a
  3072-bit key, so the defect is planted back (trap 13).
- **The corpus cannot see any of this.** Not one of `doc/pdf.js`'s documents carries a store, so
  the corpus gate asserts the *rule* — no `Good` without material — and the witnesses are a
  hierarchy built with `openssl`. The crawl is where the population is, and
  `examples/signature_algorithm_census` now counts it.

Rows: §12.8.3.3.2, §12.8.3.4.6, §12.8.3.4.7 and §12.8.3.4.8 `reported` → `partial`; §12.8.4,
§12.8.4.2, §12.8.4.4, §12.8.5 and §12.8.5.2 keep `partial` with new notes and evidence;
§12.8.3.4.4 stays `reported`, the false half of its note struck. ADR 1067.
