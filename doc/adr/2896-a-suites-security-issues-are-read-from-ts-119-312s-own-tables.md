# 2896 — A suite's security issues are read from TS 119 312's own tables

Session 2040, a signatures round (`doc/todo/77` item 3). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 1085, 2872; `doc/questions/A856`; ETSI TS 119 172-4 V1.1.1 clause 4.5; ETSI TS 119 312
V2.1.1 (2026-06) clauses 3.4, 5.1, 6.2, 7.3 and 8.4. Cited by clause and paraphrased (ADR 1085).

## Context

REQ-4.5-01 h) asks the applicability report for the suite a signature was made with and its potential
security issues against national rules or TS 119 312, naming which failed. ADR 2872 named the suite and
said no assessment was made, because TS 119 312 was not held. The owner placed V2.1.1 at
`doc/ETSI_TS_119_312_v2.1.1.pdf` (A856; gitignored, recorded in `doc/third-party-data.md`).

## Decision

`pdf_signature::verdict::applicability::ts_119_312` holds six of its tables as `const` rows, every cell
an identifier, a reference or a class: Table 1's hash functions, Table 2's signature algorithms (notes
1 and 2 as a `Condition`), Table 3's curves, Table 4's 25 suites, Tables 6 and 7's key sizes with their
end dates. `Assessment::of` asks each about one suite and `Report::security` carries the answer at the
best signature time and at the validation instant; `viewer_core::notes` names each issue's table and
clause. Readings, each a choice where the text leaves one:

1. **The classes are clause 3.4's.** `L[yyyy]` is phased out no later than 31 December of the year,
   `L[yyyy+]` on that day unless extended, a bare `L` is `L[2034+]`. A phase-out has passed from the
   first second of the next year, UTC.
2. **What an issue is.** A component no row lists (the document does not admit it), a legacy component
   (clause 4: not the state of the art; clause 8.2: a smaller margin), and a legacy component whose
   phase-out passed by the best signature time or by the validation (clause 9.2's suitability for as
   long as verification is needed). A recommended component raises none.
3. **Clause 7.3's equal lengths are a rule of their own.** An elliptic-curve suite is recommended only
   where the digest's output length is the curve's key size. Read literally this leaves P-521 with no
   recommended digest, Table 3 listing it R notwithstanding; the report says that rather than bending it.
4. **Table 4's internal hashes.** Where a row states its hash as internal and the signature used it,
   Table 1 is not asked — SHAKE256 under Ed448 is in no row of Table 1, and Table 4 admits it.
5. **Table 7 needs `q`.** Its rows bound both primes; `q` is read from the signer's certificate, and a
   key whose `q` does not read is said as not assessed.
6. **The RSASSA-PSS row is chosen by the message digest.** Table 4's two PSS rows are named by MGF1's
   hash; `Authenticity` does not carry it, so the row is the one of the digest, and that is left.

Tables 3.1, 3.2 and 3.3 (ML-DSA, SLH-DSA, hybrids) are not held: this tree computes none of those
schemes, so such a signature is REQ-4.2-03 e)'s unsupported suite before h) can ask anything.

## Consequences

`SecurityRules` is gone and `Cryptography::Suite` carries the suite only. A 2048-bit RSA PKCS #1 v1.5
signature now reports three issues — Table 2 and Table 4's bare `L` and Table 6's `L[2026]` — and from
2027 the last as passed. Left: national rules (none held, and none claimed), MGF1's hash, clause 9.3 and
9.5's key lifetimes over a certificate's validity, and clause 9.2's time-stamp imprint rule.
