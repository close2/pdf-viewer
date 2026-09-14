# 1067 — Revocation is read from the file, and absence of evidence is never `Good`

Session 1053. Status: **accepted**.

ADR 1039 built RFC 5280 section 6.1's path validation and left one step of it untaken: (a)(3),
"the certificate is not revoked". Nineteen ledger rows named that step and every one of them named
a *network* beside it, on the reading that a CRL comes from a distribution point and an OCSP
response from a responder. This ADR records that the reading was wrong about where the material
lives, decides the model that replaces it, and fixes the one rule a later round must not
re-litigate.

## 1. The decision

**Revocation is answered from the material the document itself carries, and from nothing else.**
§12.8.4's document security store exists for exactly this: "[a] PDF signature may not be
successfully verified unless its collateral validation components are preserved, e.g.,
certificates, CRLs, timestamp tokens, revocation lists, and OCSP responses" (§12.8.4.2). Table 261
carries them as streams — `/Certs`, `/CRLs`, `/OCSPs` — and §12.8.3.3.2's
`adbe-revocationInfoArchival` attribute carries the same two kinds inside the signature, under a
grammar the clause prints in full. §12.8.4.2 says the two are one supply and which comes first:
"Some of this information, i.e. certificates, CRLs and OCSP responses, when not already present in
the signature, shall be stored in a document security store (DSS)".

So the network is not what was missing. `pdf_signature::revocation` reads both supplies,
`pdf_signature::trust` applies them to every certificate on the path it has just validated, and
nothing in the crate opens a socket — which stays true by the same argument `CLAUDE.md` principle
3 makes about the sandbox.

## 2. The rule that may not be re-litigated

**`Revocation::Unknown` is the default at every level, and `Revocation::Good` is only ever the
output of arithmetic this program performed.**

A certificate no material covers, a CRL whose signature does not verify under the key that issued
the certificate, a CRL past its own `nextUpdate`, an OCSP response whose signer is not authorised
under RFC 6960 section 4.2.2.2, a list this reader could not walk to its end — every one of those
is `Unknown` with its reason kept, and none of them may become `Good`. RFC 5280 section 6.3.3 ends
the same way: "if the revocation status has still not been determined, then return the cert_status
UNDETERMINED."

The reason this is an ADR and not a comment is that the failure it prevents is silent and
attractive. A revocation check that answered `Good` when it found nothing would make every
unsigned-for document look checked, would make a missing CRL indistinguishable from a clean one,
and would read to a person as the one word this program has never said. A `Good` computed from
material nobody verified is worse than no answer at all.

**And nothing gains the word *valid*.** ADR 1039's decision stands unchanged: no host in this tree
supplies a trust anchor, so `Trust::NoAnchorSupplied` is still what every document gets, and
revocation is one more input to a verdict nobody can yet give.

## 3. Two asymmetries, both deliberate

**A `Revoked` stands where the material is stale; a `Good` does not.** A revocation is a statement
its issuer made and an expiry window does not withdraw it, while "not on this list" older than the
list's own `nextUpdate` says nothing about today. So the freshness test turns a `Good` into
`Undetermined::Stale` and leaves a `Revoked` alone. RFC 5280 section 6.3.3 (a)(1) would have the
reader fetch a newer list instead, which is the one branch of the algorithm a document cannot
supply.

**A refusal is per-piece, not per-search.** RFC 6960 section 4.2.2.2 lists three criteria for an
authorised responder and this reader takes two of them; criterion 2 is tried first, and against a
response a *delegate* signed the CA's modulus is the wrong shape rather than the wrong value — so
the arithmetic refuses by name. Treating that as the end of the search reported "this program does
not verify 1.2.840.113549.1.1.11" about an algorithm it verifies, on two real documents in the
crawl, before the census caught it. Every refusal is now local to the piece of material that
produced it and the search goes on.

## 4. The reductions, each on the standard's own permission

- **Distribution points.** A DSS names none. RFC 5280 section 6.3.3's last paragraph says what to
  do with material that arrived otherwise — "repeat the process above with any available CRLs not
  specified in a distribution point but issued by the certificate issuer. For the processing of
  such a CRL, assume a DP with both the reasons and the cRLIssuer fields omitted and a distribution
  point name of the certificate issuer" — under which step (b)(1) is a name comparison and step
  (d)(4) sets `interim_reasons_mask` to all-reasons.
- **Issuing distribution points, and every other unrecognised critical CRL extension.** Section
  5.2.5: "Although the extension is critical, conforming implementations are not required to
  support this extension. However, implementations that do not support this extension MUST either
  treat the status of any certificate not listed on this CRL as unknown or locate another CRL that
  does not contain any unrecognized critical extensions." The first branch is taken. A certificate
  the list *does* name is named whatever its scope says, so the check sits after the search.
- **Delta CRLs.** `use-deltas` is an input (section 6.3.1 (b)) and it is false.
- **Criterion 1 of RFC 6960 section 4.2.2.2** — "a local configuration of OCSP signing authority"
  — is a host's input, the same shape as a trust anchor and refused as a default for ADR 1039's
  reasons.
- **`otherRevInfo`.** §12.8.3.3.2: "[t]he format is not prescribed by this specification, other
  than that it be encoded as an OCTET STRING."

## 5. What it costs, measured rather than assumed

`examples/signature_algorithm_census` over every document this tree holds is the command; the
numbers are its output and not this file's. What the run of this session established, and what a
later round should re-derive rather than quote: a document security store is **uncommon but not
rare**, the material inside it is overwhelmingly readable, and the handful this reader refuses are
refused for one reason — a CRL larger than `der::MAX_VALUE`, which is a bound on work over a
stranger's bytes that RFC 5280 states no counterpart to (trap 38). Raising it is a separate
decision with a memory cost attached.

The dominant answer over that population is not a revocation verdict at all: most of these
signatures' certificates have expired, so section 6.1.3 (a)(2) refuses the path before (a)(3) is
reached. That is a fact about old documents rather than about this code, and it is why the census
prints the instant it asked at.

## 6. What this does not close

`doc/pdf.js`'s corpus carries **no** document security store, which is why the witnesses here are
a hierarchy this tree issued with `openssl` and why the corpus gate asserts the *rule* — no `Good`
without material — rather than a verdict. Trap 13's shape exactly: a sweep that finds nothing reads
identically as "the reader is right" and as "the reader was never called", so the defect is planted
back one case at a time in `revocation/tests.rs` and each must move the answer off `Good`.

The nineteen rows ADR 1039 named do not all become `implemented`. What changes is that "a trust
store and a network" was two debts and is now one: the network is gone, and the trust store is a
host's to supply.
