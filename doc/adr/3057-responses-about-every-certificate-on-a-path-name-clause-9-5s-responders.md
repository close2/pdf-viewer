# 3057 — Responses about every certificate on a path name clause 9.5's responders

Session 2089, a signatures round (`doc/todo/77` section 2 item 3). Status: implemented. Rows:
§12.8.3.4.5, §12.8.4.3 (read, not changed). Context: ADRs 2928 (its item 2, superseded in part),
3056; ETSI TS 119 312 V2.1.1 clause 9.5; RFC 6960 sections 4.1.1 and 4.2.2.2. Cited by clause and
paraphrased (ADR 1085).

## Context

Clause 9.5's "OCSP responder keys" are not limited to the signer's certificate's responder. The
path validation already asks §12.8.4's material about every certificate on a path (RFC 5280 section
6.1.3 (a)(3)), intermediates and a token's unit included, and decides each answer through RFC 6960
section 4.2.2.2's authorisation; but the report found a responder by its own looser test (serial
number, name or key hash, issuer name, `id-kp-OCSPSigning`), for the signer's certificate only.

## Decision

1. **The responder is the one the authorisation accepted.** `revocation::delegated_responders`
   returns, for a certificate, each delegation certificate under which `authorised` verified a
   response whose `CertID` names the certificate by all three of section 4.1.1's fields — criterion
   3, the certificate issued under the issuer's key and the response verified under its own.
   `authorised` now says which criterion held: a CA's own response names no delegate.
2. **Collected during the path walk, by position.** `trust::Responder { about, key }` is carried by
   `Trust::Anchored`, `verdict::Anchored` and `Valid`, `applicability::Asked::responders`, and
   `Time::Established`. The authorisation is computed a second time beside the status rather than
   threaded out of `status_within`; the cost is one signature check per response per path, off the
   launch path.
3. **A responder about the certificate at `about` is certified by the path's key at `about + 1`**,
   which issued that certificate; where that is the anchor the responder is asked over its own
   validity and is in no pair. `KeyRole::OcspResponder { about }` on the signer's path,
   `KeyRole::TimeStampingResponder { about }` on a token's.
4. **What a store provides is what is read.** The material is §12.8.4.3's DSS `/OCSPs` — a VRI's
   `/OCSP` being, by Table 262, a subset of it (`SecurityStore::material`) — and the archived
   `adbe-revocationInfoArchival`; nothing is fetched. A CRL issuer is the certificate's issuer here,
   so a list adds no key beyond the path's.

## Consequences

ADR 2928's responder test (by name or key hash and issuer name, unverified) is gone; a response
whose signature does not verify no longer names a key to ask. `notes` words the two new roles.
