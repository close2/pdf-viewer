# 2853 — A certificate's QC statements are read and carried, unweighed

Session 2017, a signatures round (`doc/todo/77` item 1). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 2834, 2835; RFC 3739 section 3.2.6; ETSI EN 319 412-5 V2.4.1 clauses 4.1, 4.2 and 5;
ETSI TS 119 615 V1.3.1 clause 4.4.

## Context

TS 119 615 clause 4.4 decides whether a signing certificate is an EU qualified certificate, and
its first input is what the certificate says of itself: EN 319 412-5's statements in RFC 3739's
`qcStatements` extension. Clause 4.2 defines four that claim compliance with a legislation —
`QcCompliance` (4.2.1), `QcSSCD` (4.2.2), `QcType` (4.2.3) and `QcCClegislation` (4.2.4) — and
Tables 1 and 1A give an absent `QcType` and an absent `QcCClegislation` meanings of their own.

## Decision

1. **`pdf_pki::x509` keeps the extension's octets** (`Extensions::qc_statements`) beside the path,
   as it does `extKeyUsage`: the note to clause 4.1 says the extension "is not processed as part of
   IETF RFC 5280 path validation", and QCS-4.1-02 says it "shall not be marked as critical", so a
   critical one still falls to RFC 5280 section 4.2's refusal of an unrecognised critical extension.
2. **`pdf_signature::qc_statement` reads the four** by their clauses' syntax, keeps absence as
   absence, names every other statement by its dotted identifier, refuses a statement without its
   syntax or one of the four repeated, and is bounded by `MAX_ENTRIES`; `fuzz/`'s `qc_statement`
   target holds it to that.
3. **A listing carries them** (`verdict::Listing::statements`): the statements are the signing
   certificate's, and the listing is what clause 4.4 starts from.
4. **A report names them and weighs nothing**: `viewer_core::notes` says which statements the
   certificate makes, by their ASN.1 names and clauses, as its issuer's claims, and still says the
   level is not determined (ADR 2835). Neither "qualified" nor "QES" is worded from a statement.

## Consequences

Clause 4.3's generic statements (`QcLimitValue`, `QcRetentionPeriod`, `QcPDS`) are named by
identifier and not decoded; nothing in clauses 4.4 or 4.5 reads them. Whether a two-letter
`CountryName` is an assigned ISO 3166-1 code is not checked, because that registry is not held.
TS 119 615 clause 4.4 itself — the service's qualifications extension, the status at the best
signature time and the transitions — is `doc/todo/77` item 2.
