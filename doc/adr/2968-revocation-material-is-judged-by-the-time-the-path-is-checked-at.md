# 2968 — Revocation material is judged by the clause of the time the path is checked at

Session 2073. Status: **implemented.** Context: review 2038 (findings B41, B44, B47, D36), ADRs 1067, 1071, 1085.
Clauses: ISO 32000-2 §12.8.3.4.7, §12.8.3.4.8.

## 1. Two clauses, two windows

§12.8.3.4.7, at the current time: "a CRL is appropriate only if the current UTC time is included within the time
period delimited by the thisUpdate and the nextUpdate fields". §12.8.3.4.8, at a time in the past: a CRL whose
`nextUpdate` is after that time, and an OCSP response whose "producedAt field … is located after the UTC time in
the past". ADR 1067 applied one window to every instant — ETSI EN 319 102-1 clause 5.2.5's comparison against the
next update — so a CRL issued after the reader's clock supported a signature checked at that clock, and a response
produced before a trusted past time supported a path checked at it. `revocation::Issuance` now carries a `Basis`:
`CurrentTime` adds the `thisUpdate` end, `PastTime` adds the `producedAt` condition, and `Unstated` (every other
caller, and §12.8.5's document timestamp) keeps ADR 1067's window. `notes::trust_as_of` takes the basis from the
best signature time's proof: the reader's clock, or this signature's timestamp attribute.

## 2. A compromised timestamping unit's time is not used

§12.8.3.4.8's three cases on the timestamping unit's path: a revocation for "keyCompromise" or "cACompromise", or
with no reason, and the token's time "cannot be used"; any other reason, and it is usable only if the revocation
came after the token's time. `notes::signature_timestamps_instant` answered the token's `genTime` whatever its
path's revocation said; it now declines in the first case and in the second where the revocation is not later.
