# 2905 — A PSS suite is read by its MGF1 hash, and a token's imprint by clause 9.2

Session 2044, a signatures round (`doc/todo/77` item 3). Status: implemented. Rows: §12.8.3.4.5.
Context: ADR 2896 readings 6 and its consequences; ETSI TS 119 312 V2.1.1 clauses 7.3, 9.2 and Annex D
Table D.3; RFC 4055 section 3.1. Cited by clause and paraphrased (ADR 1085).

## Decision

1. **`Family::RsaPss` carries MGF1's hash** (`mask`), the one RFC 4055 parameter Table 4 names its
   rows by. The verifier already read it (`pss::Parameters::mgf_hash`) and dropped it; it now reaches
   `Authenticity` through the family, so no other variant changes.
2. **A PSS row admits a digest and an MGF1 hash both among its hashes.** "rsa-pss with
   mgf1SHA2-Identifier" lists SHA-256, SHA-384 and SHA-512 and names MGF1 over SHA-2; the document
   defines no `mgf1SHA2-Identifier`, so the reading — a choice — is that the row's hash column bounds
   MGF1's hash as it bounds the digest. Mixed families and MGF1 over SHA-1 (Annex D Table D.3's
   discontinued `mgf1SHA-1Identifier`) are in no row, and Table 4 is the issue said.
3. **Clause 9.2's imprint rule is asked of Table 1.** The imprint hash of every token the report takes
   as a time — the signature time-stamp and each covering document time-stamp — is a component under
   `Rule::TimeStampImprints`: unlisted is `NotListed`, legacy is `Legacy`. The classes are V2.1.1's and
   move only from R towards L, so a hash legacy now was no better at an earlier `genTime`, and one
   recommended now was recommended then; `genTime` is therefore not compared. SHA-224, Table 1's only
   legacy row, is no digest this reader reads, so the rule's reachable issue is the unlisted hash.
   It is attached where the suite was assessed; a signature whose suite was not reached carries none.

## Consequences

Left of item 3: clause 9.3 and 9.5's key lifetimes over a certificate's validity, and national rules
(none held). The report's threading of imprints is pinned by a report over a document whose second
document timestamp covers the first, and dropping the call from `Report::of` fails it; a signature
timestamp's imprint takes the same path and has no report-level fixture of its own.
