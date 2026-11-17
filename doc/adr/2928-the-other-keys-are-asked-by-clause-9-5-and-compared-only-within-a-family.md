# 2928 — The other keys are asked by clause 9.5, and compared only within a family

Session 2057, a signatures round (`doc/todo/77` item 3). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 2896, 2905, 2927 (its item 5); ETSI TS 119 312 V2.1.1 (2026-06) clauses 3.1, 6.2.2.3,
8.4, 9.3, 9.4 and 9.5; ETSI TS 119 172-4 V1.1.1 REQ-4.5-01 h); RFC 6960 section 4.2.2.2. Cited by
clause and paraphrased (ADR 1085).

## Context

Clause 9.5 says the keys other than the signer's — a time-stamping unit's, a CA's, a CRL issuer's,
an OCSP responder's — should resist over their own certificate's validity and that of the
certificates relying on it, and that their security parameters shall be at least as strong as the
corresponding parameters of the keys they certify. ADR 2927 left it because the report was handed
none of those keys: `trust::Trust::Anchored` and `verdict::Valid` kept the path's length only.

## Decision

1. **The keys are carried, not the certificates.** `trust::CertifiedKey` is a key's parameters as
   Tables 2, 3, 6 and 7 name them (`KeyParameters`) and its certificate's validity, owned, because a
   certificate borrows the file and a `Trust` does not. `Trust::Anchored` gains `keys`, target first
   and the anchor excluded, so `length` of them; `verdict::Anchored` and `Valid` pass them on, and
   `applicability::Asked::path` takes them. Its meaning is unchanged. An `rsaEncryption` key's
   Table 2 row is the padding of the signature it was seen to make — the certificate below it on
   the path, the response, the token; `id-RSASSA-PSS` is RSASSA-PSS — and none where none was seen.
2. **Whose keys.** The path's certificates from position 1 up, which are also the CRL issuers, a
   CRL's issuer being the certificate's issuer in `revocation`. The anchor is clause 9.4's and is not
   asked. A delegated OCSP responder is the certificate a response about the signer's certificate
   carries, names in its `ResponderID`, issued by the signer's issuer, asserting
   `id-kp-OCSPSigning` (RFC 6960 criterion 3); a response the CA signed itself names the path's key.
   A time-stamping unit is the certificate a token's `SignerInfo` names, for each token taken as a
   time.
3. **The *should* is asked as clause 9.3 is asked** (ADR 2927 items 1 and 2): each of the key's rows
   whose phase-out has passed by the end of the period is `Issue::OtherKeyOutlived`. An authority's
   period ends at the latest `notAfter` of its own certificate and of every certificate below it on
   the path, each of which relies on it — a choice, since the clause does not say *which* relying
   certificates. A responder's and a unit's period is their own certificate's: no certificate here
   relies on theirs. A row no table lists is `OtherKeyNotListed`, an unread validity
   `OtherKeyValidityNotRead`.
4. **"At least as strong" is compared within a family only.** Corresponding parameters are an RSA
   modulus against an RSA modulus (Table 6), a DSA key's `p` and `q` against another's (Table 7),
   and a curve's key size against another's under the same scheme (clause 6.2.2.3; `EdDSA`'s fixed
   sizes likewise). Smaller in any is `Issue::WeakerThanCertified`. Across families the document
   gives no per-key figure: clause 3.1 defines the security level and bounds each class from below —
   a legacy mechanism at least 100 bits, a recommended one at least 125 — and neither bound orders
   two keys, since either class may exceed it. So such a pair is `Issue::NotCompared`, said rather
   than guessed. The pairs are each authority over the key below it, and the signer's issuer over a
   delegated responder; a unit's issuer is not carried, so a unit is in no pair.
5. **Clause 9.5's last sentence is not asked.** It applies clause 9.4's rule to these keys: secure
   for as long as the signature needs verifying. That end is not an instant the report holds.

## Consequences

`Trust::Anchored` has a third field, and the five tests that compared a whole `Trust` with a value
now match a pattern, except the one that pins the carried keys. The OCSP and time-stamp fixtures'
keys raise clause 9.5's issues: a 2048-bit PKCS #1 v1.5 unit to 2036 outlives Tables 2 and 6, and a
2048-bit CA certifying a 3072-bit responder is weaker than the key it certifies. Left: national
rules, a token's own path and responses about the path's other certificates.
