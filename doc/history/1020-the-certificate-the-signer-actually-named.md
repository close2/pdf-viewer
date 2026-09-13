# 1020 — The certificate the signer actually named

Date: 2026-09-13. Branch: `batch-1020-1025`, worktree `/home/AI/pdf-viewer-rounds`, shared with five
sibling rounds. ADR: none — nothing here is a decision a later round must not re-litigate; the
arguments live beside the code and in the four ledger rows.

## What moved

Four `partial` rows in the §12.8.3 family, and what unblocked three of them was not a reading but a
*holding*: `doc/md/rfc/` now carries the free IETF RFCs. §12.8.3.4.3 (f) names RFC 5035 for the
signing-certificate attributes, and that RFC's Appendix A module assigns three of (i)'s four
identifiers. Both rows had read "defined in documents this tree does not hold", and both were true.

- **§12.8.3.4.5 (a), both sentences.** The second — verify the digest under the key in the signer's
  certificate — since session 392. The first is new: `pdf_signature::ess` reads RFC 5035 section
  5.4.2's `SigningCertificate` and 5.4.1's `SigningCertificateV2` (including the `DEFAULT
  {algorithm id-sha256}` a DER producer omits), and `signing_certificate_bindings` hashes the
  certificate as section 5.4.1.1 fixes it: "over the entire DER-encoded certificate (including the
  signature)". A mismatch is `Authenticity::SigningCertificateMismatch`; an attribute this program
  cannot act on is `SigningCertificateUnverifiable`. Neither is ever a pass. (Its (b) moved in a
  sibling round of the same batch, which is why the row now reads as it does.)
- **§12.8.3.4.3 went from three of eleven checked to eight** — (f) and all four of (i) new, (h) new
  with a caveat, (g) found already enforced under §12.8.3.4.2's citation. (b), (c) and (j) each state
  their rule *by reference* to ETSI EN 319 122-1 and state nothing themselves.
- **§12.8.3.3 and §12.8.3.3.1** gained the half of "this certificate shall be used to verify the
  signature value" that decides *which* certificate: their reader finds it through the `SignerInfo`'s
  `sid`, and RFC 5035 section 5.4.1.1 says what is wrong with that alone — "the sid field is not
  covered by the signature."

## Three things worth keeping

**`der::Value::encoding` is the whole of why the hash is right.** A `Value` handed back its contents,
and `certHash` is over the octets the file wrote. Hashing the contents instead was planted
deliberately and turned `issue16553.pdf` — the corpus's one witness — from `Matches` to `Differs`,
and its verdict from `Verified (4096-bit RSA)` to `SigningCertificateMismatch`.

**The check is made for every CMS sub-filter, not only `ETSI.CAdES.detached`.** RFC 5035 section 5.4.1
puts the same MUST on any object carrying the attribute, and the corpus's one witness is an
`adbe.pkcs7.detached`; scoping it to the clause's own sub-filter would have left the only real file
that exercises it unchecked.

**§12.8.3.4.3 (h) is the one identifier with a single reading, and the constant says so.** ISO names
ETSI EN 319 122-1; this tree holds RFC 5126 section 5.11.2, which defines the same attribute and
prints its number. `const_oid::db::rfc5911` seconds the other five and has no row for this one. A
wrong identifier stops the rule *firing* — a report not made, not a verdict wrongly given.
