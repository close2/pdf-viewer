# 2852 — An RSASSA-PSS subject key is an RSA key restricted to PSS

Session 2017, a signatures round (`doc/todo/77` item 0). Status: implemented. Rows: §12.8.3.4.4,
§12.8.3.4.5. Context: ADRs 0322, 2842, 2843; RFC 4055 sections 1.2, 3.1 and 3.3; RFC 8017
Appendix A.2.3.

## Context

Germany's trusted list (read as evidence on 2026-11-12, never committed) is signed with RFC 6931's
`sha256-rsa-MGF1` by a certificate whose `subjectPublicKeyInfo` names `id-RSASSA-PSS`, with no
parameters. `pdf_pki::x509` read only `rsaEncryption` as an RSA key and carried every other
identifier as `PublicKey::Unverifiable`, so TS 119 615 clause 4.2 refused the list by name. RFC 4055
section 1.2 says the key is the same `RSAPublicKey` under either identifier, and that the
`id-RSASSA-PSS` identifier is its owner restricting it: the certificate user "MUST only use the
certified RSA public key for RSASSA-PSS operations" and, where `RSASSA-PSS-params` are present,
must use "the one-way hash function, mask generation function, and trailer field identified" there.
Section 3.3 says how a signature's parameters are held to a key's: absent key parameters need no
validation; present ones must match "except the saltLength field", whose signature value "MUST be
greater or equal to that in the key parameters field".

## Decision

1. **A separate variant**, `PublicKey::RsaPss { key, parameters }`, rather than `PublicKey::Rsa`
   with a flag: every verifier that matches on a key has to answer for the restriction, and a
   PKCS #1 v1.5 signature under such a key reaches no arm that verifies it (the CMS path answers
   `KeyDoesNotMatchAlgorithm` with the key named as `1.2.840.113549.1.1.10`; the certificate,
   revocation and XML paths name the algorithm; `adbe.x509.rsa_sha1` names the key).
2. **The key's parameters are carried unread** by `pdf-pki` and read by `pdf_signature::pss`, the
   one reader of `RSASSA-PSS-params`; `pss::admitted_by_key` is section 3.3, compared on the
   digests the parameters name so that an explicit SHA-1 and an absent field compare equal (section
   3.1 requires a verifier to read them alike).
3. **Every PSS verifier asks it**: CMS signer information, a certificate's or a CRL's or an OCSP
   response's own signature, and a trusted list's XML signature, whose RFC 6931 method fixes the
   hash, MGF1's hash and a salt as long as the hash.
4. **A mismatch is a refusal naming both sets of parameters**, `PssParametersNotVerifiable` in a
   signature's report, because the key's owner did not admit what was used; an unreadable key
   restriction is refused the same way.

## Consequences

RFC 4055 section 1.2's `keyUsage` constraints on such a certificate are an issuer's obligation and
are not checked here. `id-RSAES-OAEP` keys stay `Unverifiable`, which is right for a signature: the
same section restricts them to key transport. Germany's list now reaches clause 4.2's
authentication by its signature; the evidence run is in the round's record.
